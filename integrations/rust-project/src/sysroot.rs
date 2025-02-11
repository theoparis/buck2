/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::Stdio;

use anyhow::Context;
use tracing::instrument;

use crate::buck::truncate_line_ending;
use crate::buck::utf8_output;
use crate::buck::Buck;
use crate::json_project::Sysroot;

#[derive(Debug)]
pub(crate) enum SysrootConfig {
    Sysroot(PathBuf),
    BuckConfig,
    Rustup,
}

/// Choose sysroot and sysroot_src based on platform.
///
/// `sysroot` is the directory that contains std crates:
/// <https://doc.rust-lang.org/rustc/command-line-arguments.html#--sysroot-override-the-system-root>
/// and also contains libexec helpers such as rust-analyzer-proc-macro-srv.
///
/// `sysroot_src` is the directory that contains the source to std crates:
/// <https://rust-analyzer.github.io/manual.html#non-cargo-based-projects>
#[instrument(ret)]
pub(crate) fn resolve_buckconfig_sysroot(project_root: &Path) -> Result<Sysroot, anyhow::Error> {
    let buck = Buck::default();

    if cfg!(target_os = "linux") {
        let sysroot_src = project_root.join(buck.resolve_sysroot_src()?);

        // TODO(diliopoulos): remove hardcoded path to toolchain sysroot and replace with something
        // derived from buck, e.g.
        //
        // $ buck cquery -u fbcode//buck2/integrations/rust-project:rust-project -a compiler fbcode//buck2/platform/toolchain:rust_bootstrap
        // ...
        //     "compiler": "fbcode//tools/build/buck/wrappers:rust-platform010-clang-17-nosan-compiler (fbcode//buck2/platform/execution:linux-x86_64#54c5d1cbad5316cb)"
        // $ buck cquery -u fbcode//buck2/integrations/rust-project:rust-project -a exe fbcode//tools/build/buck/wrappers:rust-platform010-clang-17-nosan-compiler
        // ...
        //     "exe": "fbcode//third-party-buck/platform010/build/rust/llvm-fb-17:bin/rustc (fbcode//buck2/platform/execution:linux-x86_64#54c5d1cbad5316cb)",
        let sysroot = Sysroot {
            sysroot: project_root.join("fbcode/third-party-buck/platform010/build/rust/llvm-fb-17"),
            sysroot_src: Some(sysroot_src),
        };

        return Ok(sysroot);
    }
    // Spawn both `rustc` and `buck audit config` in parallel without blocking.
    let fbsource_rustc = project_root.join("xplat/rust/toolchain/current/basic/bin/rustc");
    let mut sysroot_cmd = Command::new(fbsource_rustc);
    sysroot_cmd
        .arg("--print=sysroot")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let sysroot_child = sysroot_cmd.spawn()?;

    let sysroot_src = project_root.join(buck.resolve_sysroot_src()?);

    // Now block while we wait for both processes.
    let mut sysroot = utf8_output(sysroot_child.wait_with_output(), &sysroot_cmd)
        .context("error asking rustc for sysroot")?;
    truncate_line_ending(&mut sysroot);

    Ok(Sysroot {
        sysroot: sysroot.into(),
        sysroot_src: Some(sysroot_src),
    })
}

#[instrument(ret)]
pub(crate) fn resolve_rustup_sysroot() -> Result<Sysroot, anyhow::Error> {
    let mut cmd = Command::new("rustc");
    cmd.arg("--print=sysroot")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut output = utf8_output(cmd.output(), &cmd)?;
    truncate_line_ending(&mut output);
    let sysroot = PathBuf::from(output);
    let sysroot_src = sysroot
        .join("lib")
        .join("rustlib")
        .join("src")
        .join("rust")
        .join("library");

    let sysroot = Sysroot {
        sysroot,
        sysroot_src: Some(sysroot_src),
    };
    Ok(sysroot)
}
