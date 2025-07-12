
use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;

mod common;
use common::setup_test_project;

#[test]
fn test_macro_def_predicate() {
    let dir = setup_test_project();
    Command::cargo_bin("rdump")
        .unwrap()
        .current_dir(dir.path())
        .arg("search")
        .arg("macro:my_macro")
        .assert()
        .success()
        .stdout(predicate::str::contains("src/macros.rs"))
        .stdout(predicate::str::contains("macro_rules! my_macro"));
}

#[test]
fn test_macro_call_predicate() {
    let dir = setup_test_project();
    Command::cargo_bin("rdump")
        .unwrap()
        .current_dir(dir.path())
        .arg("search")
        .arg("call:my_macro")
        .assert()
        .success()
        .stdout(predicate::str::contains("src/main.rs"))
        .stdout(predicate::str::contains("my_macro!"));
}
