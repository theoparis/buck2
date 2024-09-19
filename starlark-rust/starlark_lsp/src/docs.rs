/*
 * Copyright 2019 The Starlark in Rust Authors.
 * Copyright (c) Facebook, Inc. and its affiliates.
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     https://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use starlark::docs::DocFunction;
use starlark::docs::DocParam;
use starlark::docs::DocParams;
use starlark::docs::DocProperty;
use starlark::docs::DocString;
use starlark::docs::DocStringKind;
use starlark::typing::Ty;
use starlark_syntax::syntax::ast::AstAssignTargetP;
use starlark_syntax::syntax::ast::AstLiteral;
use starlark_syntax::syntax::ast::AstPayload;
use starlark_syntax::syntax::ast::AstStmtP;
use starlark_syntax::syntax::ast::DefP;
use starlark_syntax::syntax::ast::ExprP;
use starlark_syntax::syntax::ast::ParameterP;
use starlark_syntax::syntax::ast::StmtP;

/// Given the AST node for a `def` statement, return a `DocFunction` if the
/// `def` statement has a docstring as its first statement.
pub(crate) fn get_doc_item_for_def<P: AstPayload>(def: &DefP<P>) -> Option<DocFunction> {
    if let Some(doc_string) = peek_docstring(&def.body) {
        let args: Vec<_> = def
            .params
            .iter()
            .filter_map(|param| match &param.node {
                ParameterP::Normal(p, ..) | ParameterP::Args(p, _) | ParameterP::KwArgs(p, _) => {
                    Some(DocParam::Arg {
                        name: p.ident.to_owned(),
                        docs: None,
                        typ: Ty::any(),
                        default_value: None,
                    })
                }
                // TODO(nga): this should not ignore `/` and `*` markers.
                //
                _ => None,
            })
            .collect();

        let doc_function = DocFunction::from_docstring(
            DocStringKind::Starlark,
            DocParams { params: args },
            // TODO: Figure out how to get a `Ty` from the `def.return_type`.
            Ty::any(),
            Some(doc_string),
            None,
        );
        Some(doc_function)
    } else {
        None
    }
}

pub(crate) fn get_doc_item_for_assign<P: AstPayload>(
    previous_node: &AstStmtP<P>,
    _assign: &AstAssignTargetP<P>,
) -> Option<DocProperty> {
    peek_docstring(previous_node).map(|doc_string| {
        DocProperty {
            docs: DocString::from_docstring(DocStringKind::Starlark, doc_string),
            // TODO: Can constants have a type?
            typ: Ty::any(),
        }
    })
}

fn peek_docstring<P: AstPayload>(stmt: &AstStmtP<P>) -> Option<&str> {
    match &stmt.node {
        StmtP::Statements(stmts) => stmts.first().and_then(peek_docstring),
        StmtP::Expression(expr) => match &expr.node {
            ExprP::Literal(AstLiteral::String(s)) => Some(s.node.as_str()),
            _ => None,
        },
        _ => None,
    }
}
