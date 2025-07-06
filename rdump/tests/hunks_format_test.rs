use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::fs;
use std::io::Write;
use std::process::Command;
use tempfile::tempdir;

fn setup_hunks_test_dir() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempdir().unwrap();
    let root = dir.path().to_path_buf();

    let file_path = root.join("test.txt");
    let mut file = fs::File::create(&file_path).unwrap();
    writeln!(file, "line 1").unwrap();
    writeln!(file, "line 2").unwrap();
    writeln!(file, "line 3").unwrap();
    writeln!(file, "line 4").unwrap();
    writeln!(file, "line 5").unwrap();

    (dir, root)
}

#[test]
fn test_hunks_format() -> Result<(), Box<dyn std::error::Error>> {
    let (_dir, root) = setup_hunks_test_dir();

    let mut cmd = Command::cargo_bin("rdump")?;
    cmd.current_dir(&root);
    cmd.arg("search")
        .arg("contains:'line 3'")
        .arg("--format")
        .arg("hunks")
        .arg("-C")
        .arg("1");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("line 2"))
        .stdout(predicate::str::contains("line 3"))
        .stdout(predicate::str::contains("line 4"))
        .stdout(predicate::str::contains("line 1").not())
        .stdout(predicate::str::contains("line 5").not());

    Ok(())
}
