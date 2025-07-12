use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::fs;
use std::io::Write;
use std::process::Command;
use tempfile::tempdir;

fn setup_test_dir() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempdir().unwrap();
    let root = dir.path().to_path_buf();

    fs::File::create(root.join("a.log"))
        .unwrap()
        .write_all(b"[error] something went wrong")
        .unwrap();

    fs::File::create(root.join("b.txt"))
        .unwrap()
        .write_all(b"this is a test")
        .unwrap();

    fs::File::create(root.join("important.log"))
        .unwrap()
        .write_all(b"[warn] something might be wrong")
        .unwrap();

    fs::create_dir(root.join("old")).unwrap();
    fs::File::create(root.join("old/old_file.txt"))
        .unwrap()
        .write_all(b"this is an old error")
        .unwrap();

    fs::File::create(root.join("exact_size_123.bin"))
        .unwrap()
        .write_all(&[0; 123])
        .unwrap();

    (dir, root)
}

#[test]
fn test_deeply_nested_query() {
    let (_dir, root) = setup_test_dir();
    let mut cmd = Command::cargo_bin("rdump").unwrap();
    cmd.current_dir(&root);
    cmd.arg("search")
        .arg(r#"((name:"*.log" or name:"*.txt") and (contains:"error" or contains:"warn")) and not (path:"old")"#);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("a.log"))
        .stdout(predicate::str::contains("important.log"))
        .stdout(predicate::str::contains("old_file.txt").not());
}

#[test]
fn test_query_with_mixed_case_operators() {
    let (_dir, root) = setup_test_dir();
    let mut cmd = Command::cargo_bin("rdump").unwrap();
    cmd.current_dir(&root);
    cmd.arg("search").arg(r#"name:"*.log" AND contains:"error""#);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("a.log"));
}

#[test]
fn test_exact_size_query() {
    let (_dir, root) = setup_test_dir();
    let mut cmd = Command::cargo_bin("rdump").unwrap();
    cmd.current_dir(&root);
    cmd.arg("search").arg(r#"size:"=123""#);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("exact_size_123.bin"));
}

#[test]
fn test_empty_file_size_query() {
    let (_dir, root) = setup_test_dir();
    let mut cmd = Command::cargo_bin("rdump").unwrap();
    cmd.current_dir(&root);
    cmd.arg("search").arg(r#"size:"=0""#);

    cmd.assert().success().stdout(predicate::str::is_empty());
}