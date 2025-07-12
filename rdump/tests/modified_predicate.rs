
use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::fs::{self, File};
use std::process::Command;
use std::thread;
use std::time::Duration;
use tempfile::tempdir;

fn setup_test_dir() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempdir().unwrap();
    let root = dir.path().to_path_buf();

    // Create files with different modification times
    thread::sleep(Duration::from_secs(2));
    File::create(root.join("old.txt")).unwrap();
    thread::sleep(Duration::from_secs(2));
    File::create(root.join("recent.txt")).unwrap();

    (dir, root)
}

#[test]
fn test_modified_after() {
    let (_dir, root) = setup_test_dir();
    let mut cmd = Command::cargo_bin("rdump").unwrap();
    cmd.current_dir(&root);
    cmd.arg("search").arg("modified:>1s");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("recent.txt"))
        .stdout(predicate::str::contains("old.txt").not());
}

#[test]
fn test_modified_before() {
    let (_dir, root) = setup_test_dir();
    let mut cmd = Command::cargo_bin("rdump").unwrap();
    cmd.current_dir(&root);
    cmd.arg("search").arg("modified:<1s");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("old.txt"))
        .stdout(predicate::str::contains("recent.txt").not());
}

#[test]
fn test_modified_exact_date() {
    let (_dir, root) = setup_test_dir();
    let mut cmd = Command::cargo_bin("rdump").unwrap();
    cmd.current_dir(&root);
    let now = chrono::Local::now().format("%Y-%m-%d").to_string();
    cmd.arg("search").arg(format!("modified:{}", now));

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("recent.txt"))
        .stdout(predicate::str::contains("old.txt"));
}

#[test]
fn test_invalid_date_format() {
    let (_dir, root) = setup_test_dir();
    let mut cmd = Command::cargo_bin("rdump").unwrap();
    cmd.current_dir(&root);
    cmd.arg("search").arg("modified:invalid-date");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Invalid date format"));
}
