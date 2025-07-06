use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;

mod common;
use common::setup_test_project;

#[test]
fn test_def_finds_python_class() {
    let dir = setup_test_project();

    let mut cmd = Command::cargo_bin("rdump").unwrap();
    cmd.current_dir(dir.path());
    cmd.arg("search").arg("def:Helper & ext:py");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("helper.py"))
        .stdout(predicate::str::contains("```py")) // Check for markdown code fence
        .stdout(predicate::str::contains("src/main.rs").not());
}

#[test]
fn test_func_finds_python_function() {
    let dir = setup_test_project();

    let mut cmd = Command::cargo_bin("rdump").unwrap();
    cmd.current_dir(dir.path());
    cmd.arg("search").arg("func:run_helper");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("helper.py"))
        .stdout(predicate::str::contains("```py")) // Check for markdown code fence
        .stdout(predicate::str::contains("src/main.rs").not());
}

#[test]
fn test_import_finds_python_import() {
    let dir = setup_test_project();

    let mut cmd = Command::cargo_bin("rdump").unwrap();
    cmd.current_dir(dir.path());
    cmd.arg("search").arg("import:os & ext:py");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("helper.py"))
        .stdout(predicate::str::contains("```py")) // Check for markdown code fence
        .stdout(predicate::str::contains("src/lib.rs").not());
}