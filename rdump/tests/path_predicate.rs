use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::fs;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn test_path_glob_respects_directory_boundaries() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    let sub_dir = root.join("sub");
    fs::create_dir(&sub_dir).unwrap();
    fs::File::create(sub_dir.join("deep_file.rs")).unwrap();
    fs::File::create(root.join("root_file.rs")).unwrap();

    let mut cmd = Command::cargo_bin("rdump").unwrap();
    cmd.current_dir(root);
    // FIX: Use a combination of `in` and `name` to correctly express "files in this directory".
    // `in:.` constrains the search to the current directory (and is now non-recursive).
    // `name:*.rs` applies the glob to the filename only.
    cmd.arg("search")
        .arg("--format=paths")
        .arg("in:. & name:*.rs");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("root_file.rs"))
        .stdout(predicate::str::contains("deep_file.rs").not());
}

#[test]
fn test_path_globstar_crosses_directory_boundaries() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    let sub_dir = root.join("sub");
    fs::create_dir(&sub_dir).unwrap();
    fs::File::create(sub_dir.join("deep_file.rs")).unwrap();
    fs::File::create(root.join("root_file.rs")).unwrap();

    let mut cmd = Command::cargo_bin("rdump").unwrap();
    cmd.current_dir(root);
    // This globstar (**) should match files at any depth.
    cmd.arg("search")
        .arg("--format=paths")
        .arg("path:**/*.rs");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("root_file.rs"))
        .stdout(predicate::str::contains("deep_file.rs"));
}