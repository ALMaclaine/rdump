
use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;

mod common;
use common::setup_test_project;

#[test]
fn test_trait_predicate() {
    let dir = setup_test_project();
    Command::cargo_bin("rdump")
        .unwrap()
        .current_dir(dir.path())
        .arg("search")
        .arg("trait:Summary")
        .assert()
        .success()
        .stdout(predicate::str::contains("src/traits.rs"))
        .stdout(predicate::str::contains("pub trait Summary"));
}

#[test]
fn test_impl_predicate() {
    let dir = setup_test_project();
    Command::cargo_bin("rdump")
        .unwrap()
        .current_dir(dir.path())
        .arg("search")
        .arg("impl:NewsArticle")
        .assert()
        .success()
        .stdout(predicate::str::contains("src/traits.rs"))
        .stdout(predicate::str::contains("impl Summary for NewsArticle"));
}
