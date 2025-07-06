// START tests/code_aware_search.rs

use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::fs;
use std::io::Write;
use std::process::Command;
use tempfile::tempdir;

/// A helper to set up a temporary directory with a sample Rust project.
fn setup_test_project() -> tempfile::TempDir {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    fs::create_dir(&src_dir).unwrap();

    let main_rs_content = r#"
// This is the main application file.
use crate::lib::User;

struct Cli {
    pattern: String,
}

pub fn main() {
    println!("Hello, world!");
}
"#;
    let mut main_rs = fs::File::create(src_dir.join("main.rs")).unwrap();
    main_rs.write_all(main_rs_content.as_bytes()).unwrap();

    let lib_rs_content = r#"
// This is a library file.
use serde::Serialize;

pub struct User {
    id: u64,
    name: String,
}

impl User {
    pub fn new() -> Self {
        Self { id: 0, name: "".into() }
    }
}

pub enum Role {
    Admin,
    User,
}
"#;
    let mut lib_rs = fs::File::create(src_dir.join("lib.rs")).unwrap();
    lib_rs.write_all(lib_rs_content.as_bytes()).unwrap();

    let readme_md_content = "# Test Project\nThis is a README for Role and User structs.";
    let mut readme_md = fs::File::create(dir.path().join("README.md")).unwrap();
     readme_md.write_all(readme_md_content.as_bytes()).unwrap();

    // --- NEW: Add a Python file ---
    let py_content = r#"
import os

class Helper:
    def __init__(self):
        self.path = os.getcwd()

def run_helper():
    h = Helper()
    return h.path
"#;
    let mut py_file = fs::File::create(dir.path().join("helper.py")).unwrap();
    py_file.write_all(py_content.as_bytes()).unwrap();

     dir
 }

#[test]
fn test_def_finds_struct_in_correct_file() {
    let dir = setup_test_project();

    let mut cmd = Command::cargo_bin("rdump").unwrap();
    cmd.current_dir(dir.path());
    cmd.arg("search").arg("def:Cli"); // Query for the Cli struct

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("src/main.rs"))
        .stdout(predicate::str::contains("struct Cli"))
        .stdout(predicate::str::contains("pub fn main()"))
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
        .stdout(predicate::str::contains("pub enum Role"))
        .stdout(predicate::str::contains("pub struct User"))
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
        .stdout(predicate::str::contains("use serde::Serialize;"))
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
fn test_def_finds_python_class() {
    let dir = setup_test_project();

    let mut cmd = Command::cargo_bin("rdump").unwrap();
    cmd.current_dir(dir.path());
    cmd.arg("search").arg("def:Helper & ext:py");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("helper.py"))
        .stdout(predicate::str::contains("class Helper:"))
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
        .stdout(predicate::str::contains("def run_helper():"))
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
        .stdout(predicate::str::contains("import os"))
        .stdout(predicate::str::contains("src/lib.rs").not());
}
// END tests/code_aware_search.rs
