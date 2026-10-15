// SPDX-License-Identifier: GPL-3.0-only
use std::path::PathBuf;

#[test]
fn compiles_hello_example() {
    let path = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/examples/hello.syx"));
    let report = lyx_meow::compile_path(&path, None);
    assert!(report.ok, "hello should compile: {}", report.error);
    assert!(!report.ir_text.is_empty());
}

#[test]
fn missing_file_reports_fail() {
    let report = lyx_meow::compile_path(&PathBuf::from("nope-missing.syx"), None);
    assert!(!report.ok);
}
