use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::fs;
use std::io::Write;
#[cfg(unix)]
use std::os::unix::fs::symlink;
use std::process::Command;
use tempfile::tempdir;

#[test]
#[cfg(unix)] // Symlinks are best tested on Unix-like systems.
fn test_search_does_not_follow_symlinks_by_default() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    let target_file = root.join("target.txt");
    fs::write(&target_file, "content").unwrap();

    let symlink_path = root.join("link.txt");
    symlink(&target_file, &symlink_path).unwrap();

    let mut cmd = Command::cargo_bin("rdump").unwrap();
    cmd.current_dir(root);
    cmd.arg("search")
        .arg("--format=paths")
        .arg("contains:content");

    // The output should contain the real file but NOT the symlink.
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("target.txt"))
        .stdout(predicate::str::contains("link.txt").not());
}

#[test]
fn test_search_handles_invalid_utf8_file_gracefully() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    let invalid_path = root.join("invalid.bin");
    let mut file = fs::File::create(&invalid_path).unwrap();
    // Write an invalid UTF-8 byte sequence (0xC3 followed by a non-continuation byte).
    file.write_all(&[0x41, 0x42, 0xC3, 0x28, 0x43, 0x44])
        .unwrap();

    let mut cmd = Command::cargo_bin("rdump").unwrap();
    cmd.current_dir(root);
    // This query forces the tool to read the file content.
    cmd.arg("search").arg("contains:any");

    // The tool should not panic. It should report an error and exit.
    cmd.assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("Error evaluating file"));
}
