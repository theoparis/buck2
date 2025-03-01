# Copyright (c) Meta Platforms, Inc. and affiliates.
#
# This source code is licensed under both the MIT license found in the
# LICENSE-MIT file in the root directory of this source tree and the Apache
# License, Version 2.0 found in the LICENSE-APACHE file in the root directory
# of this source tree.

load(":context.bzl", "CompileContext")  # @unused: Used as type
load(":outputs.bzl", "RustcOutput")

def _analyze_llvm_lines(
        ctx: AnalysisContext,
        compile_ctx: CompileContext,
        llvm_ir_noopt: Artifact) -> list[Provider] | None:
    if compile_ctx.toolchain_info.llvm_lines_tool == None:
        return None
    llvm_lines = ctx.actions.declare_output("llvm_lines.txt")
    ctx.actions.run(
        cmd_args(
            compile_ctx.internal_tools_info.redirect_stdout,
            compile_ctx.toolchain_info.llvm_lines_tool,
            llvm_lines.as_output(),
            "llvm-lines",
            "--files",
            llvm_ir_noopt,
        ),
        category = "analyze_llvm_lines",
    )
    return [DefaultInfo(default_output = llvm_lines)]

def _llvm_time_trace(llvm_time_trace: RustcOutput) -> list[Provider]:
    return [
        DefaultInfo(
            default_output = llvm_time_trace.profile_output,
        ),
    ]

def _self_profile(self_profile: RustcOutput) -> list[Provider]:
    return [
        DefaultInfo(
            sub_targets = {
                "raw_dir": [DefaultInfo(default_output = self_profile.profile_output)],
            },
        ),
    ]

def make_profile_providers(
        ctx: AnalysisContext,
        compile_ctx: CompileContext,
        llvm_ir_noopt: Artifact,
        llvm_time_trace: RustcOutput,
        self_profile: RustcOutput) -> list[Provider]:
    sub_targets = {}

    llvm_lines = _analyze_llvm_lines(ctx, compile_ctx, llvm_ir_noopt)
    if llvm_lines != None:
        sub_targets["llvm_lines"] = llvm_lines

    sub_targets["llvm_passes"] = _llvm_time_trace(llvm_time_trace)
    sub_targets["rustc_stages"] = _self_profile(self_profile)

    return [DefaultInfo(sub_targets = sub_targets)]
