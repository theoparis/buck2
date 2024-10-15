/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

#![allow(deprecated)] // whoami::hostname is deprecated

use std::path::Path;
use std::time::Duration;

use crate::cli::Input;

#[cfg(fbcode_build)]
pub(crate) fn log_develop(duration: Duration, input: Input, invoked_by_ra: bool) {
    let mut sample = new_sample("develop");
    sample.add("duration_ms", duration.as_millis() as i64);
    sample.add("input", format!("{:?}", input));
    sample.add("revision", get_sl_revision());
    sample.add("invoked_by_ra", invoked_by_ra);
    emit_log(sample);
}

#[cfg(not(fbcode_build))]
pub(crate) fn log_develop(_duration: Duration, _input: Input, _invoked_by_ra: bool) {}

#[cfg(fbcode_build)]
pub(crate) fn log_develop_error(error: &anyhow::Error, input: Input, invoked_by_ra: bool) {
    let mut sample = new_sample("develop");
    sample.add("error", format!("{:#?}", error));
    sample.add("input", format!("{:?}", input));
    sample.add("revision", get_sl_revision());
    sample.add("invoked_by_ra", invoked_by_ra);
    emit_log(sample);
}

#[cfg(not(fbcode_build))]
pub(crate) fn log_develop_error(_error: &anyhow::Error, _input: Input, _invoked_by_ra: bool) {}

#[cfg(fbcode_build)]
fn get_sl_revision() -> String {
    std::process::Command::new("sl")
        .arg("id")
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .unwrap_or("unknown".to_owned())
}

#[cfg(fbcode_build)]
pub(crate) fn log_check(duration: Duration, saved_file: &Path, use_clippy: bool) {
    let mut sample = new_sample("check");
    sample.add("duration_ms", duration.as_millis() as i64);
    sample.add("saved_file", saved_file.display().to_string());
    sample.add("use_clippy", use_clippy.to_string());
    emit_log(sample);
}

#[cfg(not(fbcode_build))]
pub(crate) fn log_check(_duration: Duration, _saved_file: &Path, _use_clippy: bool) {}

#[cfg(fbcode_build)]
pub(crate) fn log_check_error(error: &anyhow::Error, saved_file: &Path, use_clippy: bool) {
    let mut sample = new_sample("check");
    sample.add("error", format!("{:#?}", error));
    sample.add("saved_file", saved_file.display().to_string());
    sample.add("use_clippy", use_clippy.to_string());
    emit_log(sample);
}

#[cfg(not(fbcode_build))]
pub(crate) fn log_check_error(_error: &anyhow::Error, _saved_file: &Path, _use_clippy: bool) {}

#[cfg(fbcode_build)]
fn new_sample(kind: &str) -> scuba_sample::ScubaSampleBuilder {
    let fb = fbinit::expect_init();
    let mut sample = scuba_sample::ScubaSampleBuilder::new(fb, "rust_project");
    sample.add("root_span", kind);
    sample.add("unixname", whoami::username());
    sample.add("hostname", whoami::hostname());

    // RA_PROXY_SESSION_ID is an environment variable set by the VS Code extension when it starts
    // rust-analyzer-proxy. rust-analyzer-proxy then starts rust-analyzer with the same
    // environment, and rust-analyzer invokes rust-project with the inherited environment.
    if let Ok(session_id) = std::env::var("RA_PROXY_SESSION_ID") {
        sample.add("session_id", session_id);
    }
    sample
}

#[cfg(fbcode_build)]
fn emit_log(message: scuba_sample::ScubaSampleBuilder) {
    use std::io::Write;
    use std::process::Child;
    use std::process::Command;
    use std::process::Stdio;

    let message = message.to_json().unwrap().to_string();
    let mut child: Child = match Command::new("scribe_cat")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .args(["perfpipe_rust_project"])
        .spawn()
    {
        Ok(child) => child,
        Err(_err) => {
            eprintln!("Error spawning scribe_cat child process");
            return;
        }
    };

    if child
        .stdin
        .as_mut()
        .unwrap()
        .write_all(message.as_bytes())
        .is_err()
    {
        eprintln!("Could not write to scribe_cat stdin");
    };
}
