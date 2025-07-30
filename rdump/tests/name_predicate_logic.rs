
use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::fs;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn test_exclude_spec_files_using_name_predicate() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    // Create test files
    fs::write(root.join("component.ts"), "const component = {};").unwrap();
    fs::write(
        root.join("component.spec.ts"),
        "describe('component', () => {});",
    )
    .unwrap();
    fs::write(root.join("styles.css"), "body { color: red; }").unwrap();

    let mut cmd = Command::cargo_bin("rdump").unwrap();
    cmd.current_dir(root);

    // The query should find .ts files but exclude .spec.ts files
    cmd.arg("search")
        .arg(r#"(ext:ts | ext:css) & !name:"*.spec.ts""#);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("component.ts"))
        .stdout(predicate::str::contains("styles.css"))
        .stdout(predicate::str::contains("component.spec.ts").not());
}
