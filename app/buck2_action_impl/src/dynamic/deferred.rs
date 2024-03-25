/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

use std::mem;
use std::slice;
use std::sync::Arc;

use allocative::Allocative;
use buck2_artifact::actions::key::ActionKey;
use buck2_artifact::artifact::artifact_type::Artifact;
use buck2_artifact::artifact::artifact_type::DeclaredArtifact;
use buck2_artifact::artifact::build_artifact::BuildArtifact;
use buck2_artifact::artifact::provide_outputs::ProvideOutputs;
use buck2_artifact::deferred::data::DeferredData;
use buck2_build_api::actions::key::ActionKeyExt;
use buck2_build_api::actions::RegisteredAction;
use buck2_build_api::analysis::dynamic_lambda_params::FrozenDynamicLambdaParams;
use buck2_build_api::analysis::registry::AnalysisRegistry;
use buck2_build_api::deferred::types::BaseKey;
use buck2_build_api::deferred::types::Deferred;
use buck2_build_api::deferred::types::DeferredCtx;
use buck2_build_api::deferred::types::DeferredInput;
use buck2_build_api::deferred::types::DeferredInputsRef;
use buck2_build_api::deferred::types::DeferredRegistry;
use buck2_build_api::deferred::types::DeferredValue;
use buck2_build_api::interpreter::rule_defs::artifact::associated::AssociatedArtifacts;
use buck2_build_api::interpreter::rule_defs::artifact::StarlarkArtifact;
use buck2_build_api::interpreter::rule_defs::artifact::StarlarkArtifactValue;
use buck2_build_api::interpreter::rule_defs::artifact::StarlarkDeclaredArtifact;
use buck2_build_api::interpreter::rule_defs::context::AnalysisContext;
use buck2_build_api::interpreter::rule_defs::plugins::AnalysisPlugins;
use buck2_build_api::interpreter::rule_defs::plugins::FrozenAnalysisPlugins;
use buck2_core::base_deferred_key::BaseDeferredKey;
use buck2_error::internal_error;
use buck2_error::BuckErrorContext;
use buck2_events::dispatch::get_dispatcher;
use buck2_execute::digest_config::DigestConfig;
use buck2_interpreter::error::BuckStarlarkError;
use buck2_interpreter::print_handler::EventDispatcherPrintHandler;
use dice::DiceComputations;
use dupe::Dupe;
use indexmap::IndexSet;
use starlark::collections::SmallMap;
use starlark::environment::Module;
use starlark::eval::Evaluator;
use starlark::values::dict::Dict;
use starlark::values::structs::StructRef;
use starlark::values::typing::StarlarkCallable;
use starlark::values::FrozenValueTyped;
use starlark::values::OwnedFrozenValueTyped;
use starlark::values::Value;
use starlark::values::ValueOfUnchecked;
use starlark::values::ValueTypedComplex;

use crate::dynamic::bxl::eval_bxl_for_dynamic_output;

/// The artifacts that are returned are dynamic actions, which depend on the `DynamicLambda`
/// to get their real `RegisteredAction`.
#[derive(Clone, Debug, Allocative)]
pub(crate) struct DynamicAction {
    // Points at a DynamicLambda
    input: DeferredInput,
    output_artifact_index: usize,
}

impl DynamicAction {
    pub fn new(deferred: &DeferredData<DynamicLambdaOutput>, output_artifact_index: usize) -> Self {
        Self {
            input: DeferredInput::Deferred(deferred.deferred_key().dupe()),
            output_artifact_index,
        }
    }
}

/// The lambda captured by `dynamic_output`, alongside the other required data.
#[derive(Clone, Debug, Allocative)]
pub struct DynamicLambda {
    /// the owner that defined this lambda
    owner: BaseDeferredKey,
    /// Things required by the lambda (wrapped in DeferredInput)
    dynamic: IndexSet<DeferredInput>,
    /// Things I produce
    outputs: Box<[BuildArtifact]>,
    /// A Starlark pair of the attributes and a lambda function that binds the outputs given a context
    attributes_lambda: Option<OwnedFrozenValueTyped<FrozenDynamicLambdaParams>>,
}

impl DynamicLambda {
    pub(crate) fn new(
        owner: BaseDeferredKey,
        dynamic: IndexSet<Artifact>,
        outputs: Box<[BuildArtifact]>,
    ) -> Self {
        let mut depends = IndexSet::with_capacity(dynamic.len() + 1);
        match &owner {
            BaseDeferredKey::TargetLabel(target) => {
                depends.insert(DeferredInput::ConfiguredTarget(target.dupe()));
            }
            BaseDeferredKey::BxlLabel(_) => {
                // Execution platform resolution is handled when we execute the DynamicLambda
            }
            BaseDeferredKey::AnonTarget(_) => {
                // This will return an error later, so doesn't need to have the dependency
            }
        }
        depends.extend(dynamic.into_iter().map(DeferredInput::MaterializedArtifact));
        Self {
            owner,
            dynamic: depends,
            outputs,
            attributes_lambda: None,
        }
    }

    pub(crate) fn bind(
        &mut self,
        attributes_lambda: OwnedFrozenValueTyped<FrozenDynamicLambdaParams>,
    ) -> anyhow::Result<()> {
        if self.attributes_lambda.is_some() {
            return Err(internal_error!("`attributes_lambda` field already set"));
        }
        self.attributes_lambda = Some(attributes_lambda);
        Ok(())
    }

    pub fn invoke_dynamic_output_lambda<'v>(
        eval: &mut Evaluator<'v, '_, '_>,
        lambda: Value<'v>,
        ctx: Value<'v>,
        artifacts: Value<'v>,
        outputs: Value<'v>,
    ) -> anyhow::Result<()> {
        let return_value = eval
            .eval_function(lambda, &[ctx, artifacts, outputs], &[])
            .map_err(BuckStarlarkError::new)?;

        if !return_value.is_none() {
            return Err(DynamicLambdaError::LambdaMustReturnNone(
                return_value.display_for_type_error().to_string(),
            )
            .into());
        }

        Ok(())
    }
}

/// The `Output` from `DynamicLambda`.
#[derive(Clone, Debug, Allocative)]
pub struct DynamicLambdaOutput {
    /// The actions the DynamicLambda produces, in the right order.
    /// `DynamicAction.index` is an index into this Vec.
    output: Box<[ActionKey]>,
}

impl provider::Provider for DynamicAction {
    fn provide<'a>(&'a self, _demand: &mut provider::Demand<'a>) {}
}

impl Deferred for DynamicAction {
    type Output = RegisteredAction;

    fn inputs(&self) -> DeferredInputsRef<'_> {
        DeferredInputsRef::Slice(slice::from_ref(&self.input))
    }

    async fn execute(
        &self,
        ctx: &mut dyn DeferredCtx,
        _dice: &mut DiceComputations<'_>,
    ) -> anyhow::Result<DeferredValue<Self::Output>> {
        let id = match &self.input {
            DeferredInput::Deferred(x) => x,
            _ => {
                return Err(internal_error!(
                    "DynamicAction must have a single Deferred as as inputs"
                ));
            }
        };
        let val = ctx.get_deferred_data(id)?;
        let key = val
            .downcast::<DynamicLambdaOutput>()?
            .output
            .get(self.output_artifact_index)
            .internal_error("Unexpected index in DynamicAction")?;
        Ok(DeferredValue::Deferred(key.deferred_data().dupe()))
    }
}

#[derive(Debug, buck2_error::Error)]
enum DynamicLambdaError {
    #[error("dynamic_output and anon_target cannot be used together (yet)")]
    AnonTargetIncompatible,
    #[error("dynamic_output lambda must return `None`, got: `{0}`")]
    LambdaMustReturnNone(String),
}

impl provider::Provider for DynamicLambda {
    fn provide<'a>(&'a self, demand: &mut provider::Demand<'a>) {
        demand.provide_value_with(|| ProvideOutputs(Ok(self.outputs.to_vec())));
    }
}

impl Deferred for DynamicLambda {
    type Output = DynamicLambdaOutput;

    fn inputs(&self) -> DeferredInputsRef<'_> {
        DeferredInputsRef::IndexSet(&self.dynamic)
    }

    async fn execute(
        &self,
        deferred_ctx: &mut dyn DeferredCtx,
        dice: &mut DiceComputations<'_>,
    ) -> anyhow::Result<DeferredValue<Self::Output>> {
        let output = if let BaseDeferredKey::BxlLabel(key) = &self.owner {
            eval_bxl_for_dynamic_output(key, self, deferred_ctx, dice).await
        } else {
            let env = Module::new();

            let (analysis_registry, declared_outputs) = {
                let heap = env.heap();
                let print = EventDispatcherPrintHandler(get_dispatcher());
                let mut eval = Evaluator::new(&env);
                eval.set_print_handler(&print);
                let dynamic_lambda_ctx_data = dynamic_lambda_ctx_data(self, deferred_ctx, &env)?;
                let ctx = AnalysisContext::prepare(
                    heap,
                    dynamic_lambda_ctx_data.attributes,
                    self.owner.configured_label(),
                    dynamic_lambda_ctx_data.plugins,
                    dynamic_lambda_ctx_data.registry,
                    dynamic_lambda_ctx_data.digest_config,
                );

                DynamicLambda::invoke_dynamic_output_lambda(
                    &mut eval,
                    dynamic_lambda_ctx_data.lambda.0,
                    ctx.to_value(),
                    dynamic_lambda_ctx_data.artifacts,
                    dynamic_lambda_ctx_data.outputs,
                )?;

                ctx.assert_no_promises()?;

                (ctx.take_state(), dynamic_lambda_ctx_data.declared_outputs)
            };

            let (_frozen_env, deferred) = analysis_registry.finalize(&env)?(env)?;
            let _fake_registry = mem::replace(deferred_ctx.registry(), deferred);

            let output: anyhow::Result<Vec<_>> = declared_outputs
                .into_iter()
                .map(|x| anyhow::Ok(x.ensure_bound()?.action_key().dupe()))
                .collect();
            output
        };
        Ok(DeferredValue::Ready(DynamicLambdaOutput {
            output: output?.into_boxed_slice(),
        }))
    }

    fn span(&self) -> Option<buck2_data::span_start_event::Data> {
        let owner = self.owner.to_proto().into();
        Some(buck2_data::DynamicLambdaStart { owner: Some(owner) }.into())
    }
}

/// Data used to construct an `AnalysisContext` or `BxlContext` for the dynamic lambda.
pub struct DynamicLambdaCtxData<'v> {
    pub attributes: ValueOfUnchecked<'v, StructRef<'v>>,
    pub lambda: StarlarkCallable<'v>,
    pub outputs: Value<'v>,
    pub plugins: ValueTypedComplex<'v, AnalysisPlugins<'v>>,
    pub artifacts: Value<'v>,
    pub key: &'v BaseDeferredKey,
    pub digest_config: DigestConfig,
    pub declared_outputs: IndexSet<DeclaredArtifact>,
    pub registry: AnalysisRegistry<'v>,
}

/// Sets up the data needed to create the dynamic lambda ctx and evaluate the lambda.
pub fn dynamic_lambda_ctx_data<'v>(
    dynamic_lambda: &'v DynamicLambda,
    deferred_ctx: &mut dyn DeferredCtx,
    env: &'v Module,
) -> anyhow::Result<DynamicLambdaCtxData<'v>> {
    let heap = env.heap();
    let mut outputs = SmallMap::with_capacity(dynamic_lambda.outputs.len());
    let mut declared_outputs = IndexSet::with_capacity(dynamic_lambda.outputs.len());

    let attributes_lambda = dynamic_lambda
        .attributes_lambda
        .as_ref()
        .internal_error("attributes_lambda not set")?
        .owned_as_ref(env.frozen_heap());
    let FrozenDynamicLambdaParams {
        attributes,
        plugins,
        lambda,
    } = attributes_lambda;

    let plugins: FrozenValueTyped<'static, FrozenAnalysisPlugins> = *plugins;
    let plugins: ValueTypedComplex<'v, AnalysisPlugins<'v>> =
        ValueTypedComplex::new(plugins.to_value()).internal_error("incorrect plugins type")?;

    let execution_platform = {
        match &dynamic_lambda.owner {
            BaseDeferredKey::TargetLabel(target) => {
                let configured_target = deferred_ctx.get_configured_target(target).unwrap();

                configured_target.execution_platform_resolution().dupe()
            }
            BaseDeferredKey::BxlLabel(k) => k.execution_platform_resolution().clone(),
            BaseDeferredKey::AnonTarget(_) => {
                return Err(DynamicLambdaError::AnonTargetIncompatible.into());
            }
        }
    };

    // The DeferredCtx has a registry it wants us to use as &mut.
    // The AnalysisRegistry wants ownership of a registry.
    // To overcome the difference, we create a fake registry, swap it with the one in deferred,
    // and swap back after AnalysisRegistry completes.

    let fake_registry = DeferredRegistry::new(BaseKey::Base(dynamic_lambda.owner.dupe()));

    let deferred = mem::replace(deferred_ctx.registry(), fake_registry);
    let mut registry = AnalysisRegistry::new_from_owner_and_deferred(
        dynamic_lambda.owner.dupe(),
        execution_platform,
        deferred,
    )?;
    registry.set_action_key(Arc::from(deferred_ctx.get_action_key()));

    let mut artifacts = SmallMap::with_capacity(dynamic_lambda.dynamic.len());
    let fs = deferred_ctx.project_filesystem();
    for x in &dynamic_lambda.dynamic {
        let x = match x {
            DeferredInput::MaterializedArtifact(x) => x,
            DeferredInput::ConfiguredTarget(_) => continue,
            _ => unreachable!("DynamicLambda only depends on artifact and target"),
        };
        let k = heap.alloc(StarlarkArtifact::new(x.dupe()));
        let path = deferred_ctx.get_materialized_artifact(x).unwrap();
        let v = heap.alloc(StarlarkArtifactValue::new(
            x.dupe(),
            path.to_owned(),
            fs.dupe(),
        ));
        artifacts.insert_hashed(k.get_hashed().map_err(BuckStarlarkError::new)?, v);
    }

    for x in &*dynamic_lambda.outputs {
        let k = heap.alloc(StarlarkArtifact::new(Artifact::from(x.dupe())));
        let declared = registry.declare_dynamic_output(x.get_path().dupe(), x.output_type());
        declared_outputs.insert(declared.dupe());
        let v = heap.alloc(StarlarkDeclaredArtifact::new(
            None,
            declared,
            AssociatedArtifacts::new(),
        ));
        outputs.insert_hashed(k.get_hashed().map_err(BuckStarlarkError::new)?, v);
    }

    let artifacts = Dict::new(artifacts);
    let outputs = Dict::new(outputs);

    Ok(DynamicLambdaCtxData {
        attributes: ValueOfUnchecked::new_checked(attributes.to_value())?,
        lambda: lambda.to_callable(),
        plugins,
        outputs: heap.alloc(outputs),
        artifacts: heap.alloc(artifacts),
        key: &dynamic_lambda.owner,
        digest_config: deferred_ctx.digest_config(),
        declared_outputs,
        registry,
    })
}
