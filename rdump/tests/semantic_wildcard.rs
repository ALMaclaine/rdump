use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;

#[test]
fn test_semantic_wildcard_matches_any_struct() {
    let mut cmd = Command::cargo_bin("rdump").unwrap();
    cmd.arg("search")
        .arg("--root")
        .arg("../insane_test_bed")
        .arg("--format=paths")
        .arg("struct:."); // The "." is the "match any" wildcard

    // Should find code.rs (has MyStruct) but not files that only use it
    // or files without structs like calls.rs or enum.rs
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("code.rs"))
        .stdout(predicate::str::contains("complex_query.rs").not())
        .stdout(predicate::str::contains("calls.rs").not())
        .stdout(predicate::str::contains("enum.rs").not());
}

#[test]
fn test_semantic_wildcard_matches_any_function() {
    let mut cmd = Command::cargo_bin("rdump").unwrap();
    cmd.arg("search")
        .arg("--root")
        .arg("../insane_test_bed")
        .arg("--format=paths")
        .arg("func:."); // Match any function

    // Should find all files with function definitions
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("calls.rs"))
        .stdout(predicate::str::contains("code.rs"))
        .stdout(predicate::str::contains("complex_query.rs"))
        .stdout(predicate::str::contains("same_file_def_call.rs"))
        .stdout(predicate::str::contains("trait.rs"))
        .stdout(predicate::str::contains("enum.rs").not()); // enums have variants, not funcs
}
