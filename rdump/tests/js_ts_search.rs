use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;

mod common;
use common::setup_test_project;

#[test]
fn test_def_finds_javascript_class() {
    let dir = setup_test_project();

    let mut cmd = Command::cargo_bin("rdump").unwrap();
    cmd.current_dir(dir.path());
    cmd.arg("search").arg("def:OldLogger");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("logger.js"))
        .stdout(predicate::str::contains("class OldLogger"))
        .stdout(predicate::str::contains("log_utils.ts").not());
}

#[test]
fn test_def_finds_typescript_interface_and_type() {
    let dir = setup_test_project();
    let mut cmd = Command::cargo_bin("rdump").unwrap();
    cmd.current_dir(dir.path())
        .arg("search")
        .arg("def:ILog | def:LogLevel");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("log_utils.ts"))
        .stdout(predicate::str::contains("interface ILog"))
        .stdout(predicate::str::contains(
            r#"type LogLevel = "info" | "warn" | "error";"#,
        ));
}

#[test]
fn test_func_finds_typescript_function() {
    let dir = setup_test_project();
    let mut cmd = Command::cargo_bin("rdump").unwrap();
    cmd.current_dir(dir.path())
        .arg("search")
        .arg("func:createLog");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("log_utils.ts"))
        .stdout(predicate::str::contains("export function createLog"));
}

#[test]
fn test_import_finds_typescript_import() {
    let dir = setup_test_project();
    let mut cmd = Command::cargo_bin("rdump").unwrap();
    cmd.current_dir(dir.path())
        .arg("search")
        .arg("import:path & ext:ts");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("log_utils.ts"))
        .stdout(predicate::str::contains("import * as path from 'path';"));
}
