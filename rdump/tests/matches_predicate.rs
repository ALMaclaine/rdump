use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::fs;
use std::io::Write;
use std::process::Command;
use tempfile::tempdir;

fn setup_test_dir() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempdir().unwrap();
    let root = dir.path().to_path_buf();

    fs::File::create(root.join("file1.txt"))
        .unwrap()
        .write_all(b"hello world\n(hello world)\nHELLO WORLD")
        .unwrap();

    fs::File::create(root.join("file2.txt"))
        .unwrap()
        .write_all(b"this is a test\nfoo bar baz")
        .unwrap();

    fs::File::create(root.join("unicode.txt"))
        .unwrap()
        .write_all("こんにちは世界\n你好世界".as_bytes())
        .unwrap();

    (dir, root)
}

#[test]
fn test_matches_with_special_characters() {
    let (_dir, root) = setup_test_dir();
    let mut cmd = Command::cargo_bin("rdump").unwrap();
    cmd.current_dir(&root);
    cmd.arg("search")
        .arg(r#"matches:'\(hello world\)'"#);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("file1.txt"));
}

#[test]
fn test_matches_multiple_lines() {
    let (_dir, root) = setup_test_dir();
    let mut cmd = Command::cargo_bin("rdump").unwrap();
    cmd.current_dir(&root);
    cmd.arg("search")
        .arg(r#"matches:'hello world'"#);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("hello world\n(hello world)"));
}

#[test]
fn test_matches_no_match() {
    let (_dir, root) = setup_test_dir();
    let mut cmd = Command::cargo_bin("rdump").unwrap();
    cmd.current_dir(&root);
    cmd.arg("search")
        .arg(r#"matches:'goodbye world'"#);

    cmd.assert().success().stdout(predicate::str::is_empty());
}

#[test]
fn test_matches_case_insensitive() {
    let (_dir, root) = setup_test_dir();
    let mut cmd = Command::cargo_bin("rdump").unwrap();
    cmd.current_dir(&root);
    cmd.arg("search")
        .arg(r#"matches:'(?i)hello world'"#);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("file1.txt"))
        .stdout(predicate::str::contains("hello world\n(hello world)\nHELLO WORLD"));
}

#[test]
fn test_matches_unicode() {
    let (_dir, root) = setup_test_dir();
    let mut cmd = Command::cargo_bin("rdump").unwrap();
    cmd.current_dir(&root);
    cmd.arg("search")
        .arg(r#"matches:'こんにちは世界'"#);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("unicode.txt"));
}

#[test]
fn test_invalid_regex() {
    let (_dir, root) = setup_test_dir();
    let mut cmd = Command::cargo_bin("rdump").unwrap();
    cmd.current_dir(&root);
    cmd.arg("search").arg(r#"matches:'('"#);

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("regex parse error"));
}
