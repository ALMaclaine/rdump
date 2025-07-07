use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;

mod common;
use common::setup_test_project;

#[test]
fn test_class_predicate_java() {
    let dir = setup_test_project();
    Command::cargo_bin("rdump")
        .unwrap()
        .current_dir(dir.path())
        .arg("search")
        .arg("class:Application & ext:java")
        .assert()
        .success()
        .stdout(predicate::str::contains("Application.java"))
        .stdout(predicate::str::contains("public class Application"));
}

#[test]
fn test_func_and_call_predicates_java() {
    let dir = setup_test_project();
    Command::cargo_bin("rdump")
        .unwrap()
        .current_dir(dir.path())
        .arg("search")
        .arg("func:main & call:println")
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

#[test]
fn test_import_and_comment_predicates_java() {
    let dir = setup_test_project();
    Command::cargo_bin("rdump")
        .unwrap()
        .current_dir(dir.path())
        .arg("search")
        .arg("import:ArrayList & comment:HACK")
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

#[test]
fn test_str_predicate_java() {
    let dir = setup_test_project();
    Command::cargo_bin("rdump")
        .unwrap()
        .current_dir(dir.path())
        .arg("search")
        .arg("str:\"Hello from Java!\"")
        .assert()
        .success()
        .stdout(predicate::str::contains("Application.java"));
}
