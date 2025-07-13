use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;

#[test]
fn test_distinguishes_function_call_from_definition() {
    let mut cmd = Command::cargo_bin("rdump").unwrap();
    cmd.arg("search")
        .arg("--root")
        .arg("../insane_test_bed")
        .arg("--format=hunks")
        .arg("call:my_func & path:same_file_def_call.rs");

    // The output should contain the CALL line but not the DEFINITION line.
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("my_func();"))
        .stdout(predicate::str::contains("fn my_func()").not());
}

#[test]
fn test_and_combination_of_hunk_and_boolean_predicates() {
    let mut cmd = Command::cargo_bin("rdump").unwrap();
    cmd.arg("search")
        .arg("--root")
        .arg("../insane_test_bed")
        .arg("--format=hunks")
        .arg("struct:MyStruct & ext:rs & path:code.rs");

    // The output should be just the struct hunk, not the whole file.
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("struct MyStruct"))
        .stdout(predicate::str::contains("fn my_func").not());
}

#[test]
fn test_or_combination_with_negation() {
    let mut cmd = Command::cargo_bin("rdump").unwrap();
    cmd.arg("search")
        .arg("--root")
        .arg("../insane_test_bed")
        .arg("--format=paths")
        .arg("contains:foo | !contains:baz");

    // Should find logical1.rs (contains foo) and logical2.rs (does not contain baz),
    // but not logical3.rs (because it contains baz).
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("logical1.rs"))
        .stdout(predicate::str::contains("logical2.rs"))
        .stdout(predicate::str::contains("logical3.rs").not());
}

#[test]
fn test_graceful_failure_on_non_existent_root_path() {
    let mut cmd = Command::cargo_bin("rdump").unwrap();
    cmd.arg("search")
        .arg(".")
        .arg("--root")
        .arg("/path/that/absolutely/does/not/exist");

    // Should fail with a clear error message.
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains(
            "root path '/path/that/absolutely/does/not/exist' does not exist",
        ));
}

#[test]
fn test_behavior_on_unknown_predicate() {
    let mut cmd = Command::cargo_bin("rdump").unwrap();
    // Search for a predicate that doesn't exist.
    cmd.arg("search")
        .arg("--root")
        .arg("../insane_test_bed")
        .arg("nonexistent:predicate");

    // The current behavior is to silently treat this as a "true" match for that predicate.
    // Therefore, the command succeeds and finds all files. This test documents that behavior.
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("code.rs"))
        .stdout(predicate::str::contains("calls.rs"));
}

#[test]
fn test_negation_of_hunk_predicate_produces_boolean_match() {
    // Case 1: Negating a predicate that DOES match the file.
    // The file code.rs contains `struct MyStruct`.
    // `!struct:MyStruct` should evaluate to false for this file.
    let mut cmd1 = Command::cargo_bin("rdump").unwrap();
    cmd1.arg("search")
        .arg("--root")
        .arg("../insane_test_bed")
        .arg("!struct:MyStruct & name:code.rs");

    cmd1.assert().success().stdout(predicate::str::is_empty());

    // Case 2: Negating a predicate that does NOT match the file.
    // The file code.rs does NOT contain `struct NonExistent`.
    // `!struct:NonExistent` should evaluate to true for this file.
    let mut cmd2 = Command::cargo_bin("rdump").unwrap();
    cmd2.arg("search")
        .arg("--root")
        .arg("../insane_test_bed")
        .arg("!struct:NonExistent & name:code.rs");

    cmd2.assert()
        .success()
        .stdout(predicate::str::contains("File: ../insane_test_bed/code.rs"));
}
