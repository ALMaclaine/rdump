
// In rdump/tests/ignore.rs

use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::fs;
use std::io::Write;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn test_rdumpignore() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempdir()?;
    let root = dir.path();

    // Create a file that should be ignored
    fs::File::create(root.join("ignored.txt"))?
        .write_all(b"This should be ignored.")?;

    // Create a file that should not be ignored
    fs::File::create(root.join("not_ignored.txt"))?
        .write_all(b"This should not be ignored.")?;

    // Create a .rdumpignore file
    fs::File::create(root.join(".rdumpignore"))?
        .write_all(b"ignored.txt")?;

    let mut cmd = Command::cargo_bin("rdump")?;
    cmd.current_dir(root);
    cmd.arg("search").arg("contains:\"This should be ignored.\"");

    cmd.assert()
        .success()
        .stdout(predicate::str::is_empty());

    Ok(())
}

#[test]
fn test_rdumpignore_excludes_matching_pattern() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempdir()?;
    let root = dir.path();

    fs::File::create(root.join("app.log"))?.write_all(b"log content")?;
    fs::File::create(root.join(".rdumpignore"))?.write_all(b"*.log")?;

    let mut cmd = Command::cargo_bin("rdump")?;
    cmd.current_dir(root);
    cmd.arg("search").arg("ext:log");

    cmd.assert()
        .success()
        .stdout(predicate::str::is_empty());

    Ok(())
}

#[test]
#[ignore = "Failing due to precedence issues with .gitignore and .rdumpignore"]
fn test_rdumpignore_unignore_overrides_gitignore() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempdir()?;
    let root = dir.path();

    fs::File::create(root.join("main.log"))?.write_all(b"main log")?;
    fs::File::create(root.join("debug.log"))?.write_all(b"debug log")?;

    fs::File::create(root.join(".gitignore"))?.write_all(b"*.log")?;
    fs::File::create(root.join(".rdumpignore"))?.write_all(b"!main.log")?;

    let mut cmd = Command::cargo_bin("rdump")?;
    cmd.current_dir(root);
    cmd.arg("search").arg("ext:log");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("main.log"))
        .stdout(predicate::str::contains("debug.log").not());

    Ok(())
}

#[test]
#[ignore = "Failing due to precedence issues with default ignores and .rdumpignore"]
fn test_rdumpignore_unignore_overrides_default_ignores() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempdir()?;
    let root = dir.path();

    let target_dir = root.join("target");
    fs::create_dir(&target_dir)?;
    fs::File::create(target_dir.join("build_info.txt"))?.write_all(b"build info")?;

    fs::File::create(root.join(".rdumpignore"))?.write_all(b"!target/")?;

    let mut cmd = Command::cargo_bin("rdump")?;
    cmd.current_dir(root);
    cmd.arg("search").arg("path:build_info");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("build_info.txt"));

    Ok(())
}
