

// In rdump/tests/extended_cli.rs

use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::fs;
use std::io::Write;
use std::process::Command;
use tempfile::tempdir;

/// Helper to create a directory with a few files for testing complex queries.
fn setup_query_test_dir() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempdir().unwrap();
    let root = dir.path().to_path_buf();

    // A file that will match multiple criteria (38 bytes)
    fs::File::create(root.join("main.rs"))
        .unwrap()
        .write_all(b"// main file\nfn main() {\n    println!(\"hello\");\n}")
        .unwrap();

    // A file with a different name but similar content (44 bytes)
    fs::File::create(root.join("utils.rs"))
        .unwrap()
        .write_all(b"// utility file\nfn helper() {\n    println!(\"world\");\n}")
        .unwrap();

    // A file to test case-insensitivity (8 bytes)
    fs::File::create(root.join("README.md"))
        .unwrap()
        .write_all(b"# Readme")
        .unwrap();

    (dir, root)
}

#[test]
fn test_complex_query_with_grouping() -> Result<(), Box<dyn std::error::Error>> {
    let (_dir, root) = setup_query_test_dir();

    let mut cmd = Command::cargo_bin("rdump")?;
    cmd.current_dir(&root);
    // Query: (contains:"main" or contains:"utility") and ext:rs
    cmd.arg("search")
        .arg("(contains:main | contains:utility) & ext:rs");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("main.rs"))
        .stdout(predicate::str::contains("utils.rs"))
        .stdout(predicate::str::contains("README.md").not());

    Ok(())
}

#[test]
fn test_name_predicate_case_insensitivity() -> Result<(), Box<dyn std::error::Error>> {
    let (_dir, root) = setup_query_test_dir();

    let mut cmd = Command::cargo_bin("rdump")?;
    cmd.current_dir(&root);
    // Query: name should match "readme.md" case-insensitively by default
    cmd.arg("search").arg("name:readme.md");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("README.md"));

    Ok(())
}

#[test]
fn test_matches_predicate_with_regex() -> Result<(), Box<dyn std::error::Error>> {
    let (_dir, root) = setup_query_test_dir();

    let mut cmd = Command::cargo_bin("rdump")?;
    cmd.current_dir(&root);
    // Query: matches a regex pattern for "hello" or "world"
    cmd.arg("search").arg("matches:\"(hello|world)\"");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("main.rs"))
        .stdout(predicate::str::contains("utils.rs"))
        .stdout(predicate::str::contains("README.md").not());

    Ok(())
}

#[test]
fn test_size_predicate() -> Result<(), Box<dyn std::error::Error>> {
    let (_dir, root) = setup_query_test_dir();

    // Test for files greater than 40 bytes
    let mut cmd_gt = Command::cargo_bin("rdump")?;
    cmd_gt.current_dir(&root);
    cmd_gt.arg("search").arg("size:>40b").arg("--format=paths");

    let output_gt = cmd_gt.output()?.stdout;
    let output_gt_str = String::from_utf8_lossy(&output_gt);
    assert_eq!(output_gt_str.lines().count(), 2, "Expected 2 files greater than 40 bytes");


    // Test for files less than 40 bytes
    let mut cmd_lt = Command::cargo_bin("rdump")?;
    cmd_lt.current_dir(&root);
    cmd_lt.arg("search").arg("size:<40b").arg("--format=paths");

    let output_lt = cmd_lt.output()?.stdout;
    let output_lt_str = String::from_utf8_lossy(&output_lt);
    assert_eq!(output_lt_str.lines().count(), 1, "Expected 1 file less than 40 bytes");

    Ok(())
}

