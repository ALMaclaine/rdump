use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;

mod common;
use common::setup_test_project;

#[test]
fn test_struct_predicate_go() {
    let dir = setup_test_project();
    Command::cargo_bin("rdump")
        .unwrap()
        .current_dir(dir.path())
        .arg("search")
        .arg("struct:Server & ext:go")
        .assert()
        .success()
        .stdout(predicate::str::contains("src/main.go"))
        .stdout(predicate::str::contains("type Server struct"));
}

#[test]
fn test_func_and_call_predicates_go() {
    let dir = setup_test_project();
    Command::cargo_bin("rdump")
        .unwrap()
        .current_dir(dir.path())
        .arg("search")
        .arg("func:NewServer | call:NewServer")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "func NewServer(addr string) *Server",
        ))
        .stdout(predicate::str::contains("server := NewServer(\":8080\")"));
}

#[test]
fn test_import_and_comment_predicates_go() {
    let dir = setup_test_project();
    Command::cargo_bin("rdump")
        .unwrap()
        .current_dir(dir.path())
        .arg("search")
        .arg("import:fmt & comment:\"HTTP server\"")
        .assert()
        .success()
        .stdout(predicate::str::contains("src/main.go"));
}
