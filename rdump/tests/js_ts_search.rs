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
        .stdout(predicate::str::contains("```")) // Check for markdown code fence
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

#[test]
fn test_call_predicate_javascript() {
    let dir = setup_test_project();
    Command::cargo_bin("rdump")
        .unwrap()
        .current_dir(dir.path())
        .arg("search")
        .arg("call:log & ext:js")
        .assert()
        .success()
        .stdout(predicate::str::contains("logger.js"))
        .stdout(predicate::str::contains("logger.log(\"init\");"));
}

#[test]
fn test_call_predicate_typescript() {
    let dir = setup_test_project();
    Command::cargo_bin("rdump")
        .unwrap()
        .current_dir(dir.path())
        .arg("search")
        .arg("call:log & ext:ts")
        .assert()
        .success()
        .stdout(predicate::str::contains("log_utils.ts"))
        .stdout(predicate::str::contains("console.log(newLog);"));
}

#[test]
fn test_comment_predicate_typescript() {
    let dir = setup_test_project();
    Command::cargo_bin("rdump")
        .unwrap()
        .current_dir(dir.path())
        .arg("search")
        .arg("comment:REVIEW")
        .assert()
        .success()
        .stdout(predicate::str::contains("log_utils.ts"));
}

#[test]
fn test_str_predicate_javascript() {
    let dir = setup_test_project();
    Command::cargo_bin("rdump")
        .unwrap()
        .current_dir(dir.path())
        .arg("search")
        .arg("str:logging:")
        .assert()
        .success()
        .stdout(predicate::str::contains("logger.js"));
}

#[test]
fn test_interface_and_type_predicates_typescript() {
    let dir = setup_test_project();
    Command::cargo_bin("rdump")
        .unwrap()
        .current_dir(dir.path())
        .arg("search")
        .arg("interface:ILog & type:LogLevel")
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}
