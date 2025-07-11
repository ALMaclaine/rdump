
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
