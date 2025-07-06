use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;

mod common;
use common::setup_test_project;

#[test]
fn test_def_finds_struct_in_correct_file() {
    let dir = setup_test_project();

    let mut cmd = Command::cargo_bin("rdump").unwrap();
    cmd.current_dir(dir.path());
    cmd.arg("search").arg("def:Cli"); // Query for the Cli struct

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("src/main.rs"))
        .stdout(predicate::str::contains("```rs")) // Check for markdown code fence
        .stdout(predicate::str::contains("src/lib.rs").not());
}

#[test]
fn test_def_finds_enum_in_correct_file() {
    let dir = setup_test_project();

    let mut cmd = Command::cargo_bin("rdump").unwrap();
    cmd.current_dir(dir.path());
    cmd.arg("search").arg("def:Role"); // Query for the Role enum

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("src/lib.rs"))
        .stdout(predicate::str::contains("```rs")) // Check for markdown code fence
        .stdout(predicate::str::contains("src/main.rs").not());
}

#[test]
fn test_def_with_ext_predicate_and_paths_format() {
    let dir = setup_test_project();
    let root = dir.path();

    let mut cmd = Command::cargo_bin("rdump").unwrap();
    cmd.current_dir(root);
    cmd.arg("search").arg("def:User & ext:rs");
    cmd.arg("--format=paths");

    // Normalize path for cross-platform compatibility
    let expected_path_str = format!("src{}lib.rs", std::path::MAIN_SEPARATOR);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains(expected_path_str));
}

#[test]
fn test_def_returns_no_matches_for_non_existent_item() {
    let dir = setup_test_project();

    let mut cmd = Command::cargo_bin("rdump").unwrap();
    cmd.current_dir(dir.path());
    cmd.arg("search").arg("def:NonExistent");

    // Should succeed with no output
    cmd.assert().success().stdout(predicate::str::is_empty());
}

#[test]
fn test_def_does_not_match_in_non_rust_files() {
    let dir = setup_test_project();

    let mut cmd = Command::cargo_bin("rdump").unwrap();
    cmd.current_dir(dir.path());
    // The README.md contains the words "Role" and "User"
    cmd.arg("search").arg("def:Role | def:User");

    // It should ONLY find src/lib.rs, not README.md
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("src/lib.rs"))
        .stdout(predicate::str::contains("README.md").not());
}

#[test]
fn test_func_finds_standalone_function() {
    let dir = setup_test_project();

    let mut cmd = Command::cargo_bin("rdump").unwrap();
    cmd.current_dir(dir.path());
    cmd.arg("search").arg("func:main");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("src/main.rs"))
        .stdout(predicate::str::contains("src/lib.rs").not());
}

#[test]
fn test_func_finds_impl_method() {
    let dir = setup_test_project();

    let mut cmd = Command::cargo_bin("rdump").unwrap();
    cmd.current_dir(dir.path());
    cmd.arg("search").arg("func:new");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("src/lib.rs"))
        .stdout(predicate::str::contains("src/main.rs").not());
}

#[test]
fn test_import_finds_use_statement() {
    let dir = setup_test_project();

    let mut cmd = Command::cargo_bin("rdump").unwrap();
    cmd.current_dir(dir.path());
    cmd.arg("search").arg("import:serde");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("src/lib.rs"))
        .stdout(predicate::str::contains("```rs")) // Check for markdown code fence
        .stdout(predicate::str::contains("src/main.rs").not());
}

#[test]
fn test_logical_or_across_files() {
    let dir = setup_test_project();

    let mut cmd = Command::cargo_bin("rdump").unwrap();
    cmd.current_dir(dir.path());
    cmd.arg("search").arg("func:main | import:serde");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("src/main.rs"))
        .stdout(predicate::str::contains("src/lib.rs"));
}

#[test]
fn test_comment_predicate_rust() {
    let dir = setup_test_project();
    Command::cargo_bin("rdump").unwrap()
        .current_dir(dir.path())
        .arg("search")
        .arg("comment:TODO")
        .assert()
        .success()
        .stdout(predicate::str::contains("src/main.rs"))
        .stdout(predicate::str::contains("src/lib.rs").not());
}

#[test]
fn test_str_predicate_rust() {
    let dir = setup_test_project();
    Command::cargo_bin("rdump").unwrap()
        .current_dir(dir.path())
        .arg("search")
        .arg("str:\"Hello, world!\"")
        .assert()
        .success()
        .stdout(predicate::str::contains("src/main.rs"));
}

#[test]
fn test_type_and_struct_predicates_rust() {
    let dir = setup_test_project();
    Command::cargo_bin("rdump").unwrap()
        .current_dir(dir.path())
        .arg("search")
        .arg("type:UserId & struct:User")
        .assert()
        .success()
        .stdout(predicate::str::contains("src/lib.rs"));
}