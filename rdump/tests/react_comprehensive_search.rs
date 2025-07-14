use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;

// This test suite queries the `insane_test_bed/react_comprehensive.tsx` file.

fn rdump_search() -> Command {
    let mut cmd = Command::cargo_bin("rdump").unwrap();
    cmd.arg("search")
        .arg("--root")
        .arg("../insane_test_bed")
        .arg("--format=hunks");
    cmd
}

#[test]
fn test_finds_class_component() {
    rdump_search()
        .arg("path:react_comprehensive.tsx & component:ClassComponent")
        .assert()
        .success()
        .stdout(predicate::str::contains("export class ClassComponent"));
}

#[test]
fn test_finds_memoized_component() {
    rdump_search()
        .arg("path:react_comprehensive.tsx & component:MemoizedComponent")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "export const MemoizedComponent = React.memo",
        ));
}

#[test]
fn test_finds_all_custom_hook_definitions() {
    let mut cmd = rdump_search();
    cmd.arg("path:react_comprehensive.tsx & customhook:."); // Wildcard match
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();

    // Should find both useCounter and useWindowWidth
    assert!(stdout.contains("function useCounter("));
    assert!(stdout.contains("const useWindowWidth = () =>"));
    assert_eq!(stdout.matches("File:").count(), 1); // Both in the same file
}

#[test]
fn test_finds_specific_custom_hook_definition() {
    rdump_search()
        .arg("path:react_comprehensive.tsx & customhook:useWindowWidth")
        .assert()
        .success()
        .stdout(predicate::str::contains("const useWindowWidth = () =>"))
        .stdout(predicate::str::contains("function useCounter").not());
}

#[test]
fn test_finds_all_hook_calls() {
    let mut cmd = rdump_search();
    cmd.arg("path:react_comprehensive.tsx & hook:."); // Wildcard match
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();

    // Should find all hook calls
    assert!(stdout.contains("useState(initialValue)"));
    assert!(stdout.contains("useState(window.innerWidth)"));
    assert!(stdout.contains("useEffect(() => {"));
    assert!(stdout.contains("useMemo(() => {"));
    assert!(stdout.contains("useCounter(10)"));
    assert!(stdout.contains("useWindowWidth()"));
    assert!(stdout.contains("useRef<HTMLInputElement>(null)"));
    assert!(stdout.contains("useContext(ThemeContext)"));
    assert!(stdout.contains("useCallback(() => {"));
}

#[test]
fn test_finds_specific_built_in_hook_call() {
    rdump_search()
        .arg("path:react_comprehensive.tsx & hook:useEffect")
        .assert()
        .success()
        .stdout(predicate::str::contains("useEffect(() => {"));
}

#[test]
fn test_finds_jsx_element_and_prop() {
    rdump_search()
        .arg("path:react_comprehensive.tsx & element:input & prop:id")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            r#"<input ref={inputRef} id="test-input" type="text" />"#,
        ));
}

#[test]
fn test_finds_custom_component_element() {
    rdump_search()
        .arg("path:react_comprehensive.tsx & element:ClassComponent")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            r#"<ClassComponent title="Class Component Title" />"#,
        ));
}

#[test]
fn test_finds_namespaced_svg_element() {
    // Note: The current tree-sitter grammar for TSX might parse `SVG.Circle`
    // as a single identifier `SVG.Circle` or as a member expression.
    // The query `element:SVG.Circle` should work if it's parsed as an identifier.
    // If it's a member expression, a more complex query might be needed,
    // but the current implementation handles identifiers well.
    rdump_search()
        .arg("path:react_comprehensive.tsx & element:SVG.Circle")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            r#"<SVG.Circle cx="50" cy="50" r="40" stroke="green" fill="yellow" />"#,
        ));
}

#[test]
fn test_negation_of_react_predicate() {
    // Find files that are TSX but DO NOT define a 'ClassComponent'
    let mut cmd = Command::cargo_bin("rdump").unwrap();
    cmd.arg("search")
        .arg("--root")
        .arg("../insane_test_bed")
        .arg("--format=paths")
        .arg("ext:tsx & !component:ClassComponent");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("react_comprehensive.tsx").not())
        // It should find App.tsx which is a .tsx file but doesn't define ClassComponent
        .stdout(predicate::str::contains("App.tsx"));
}

#[test]
fn test_find_component_with_specific_hook() {
    // Find a component that uses the `useMemo` hook.
    rdump_search()
        .arg("path:react_comprehensive.tsx & component:MemoizedComponent & hook:useMemo")
        .assert()
        .success()
        // The output should contain the whole component definition because both predicates match the file
        // and the hunks are combined.
        .stdout(predicate::str::contains("export const MemoizedComponent"))
        .stdout(predicate::str::contains("const calculated = useMemo("));
}

#[test]
fn test_find_jsx_comment() {
    // Note: Tree-sitter grammars might parse JSX comments differently from regular comments.
    // This tests if the `comment` predicate is correctly configured for JSX contexts.
    // TSX grammar does not have a dedicated `jsx_comment` node, it's just `comment`.
    rdump_search()
        .arg("path:react_comprehensive.tsx & comment:\"A JSX comment\"")
        .assert()
        .success()
        .stdout(predicate::str::contains("// A JSX comment"));
}

#[test]
fn test_find_prop_with_boolean_value() {
    rdump_search()
        .arg("path:react_comprehensive.tsx & element:button & prop:disabled")
        .assert()
        .success()
        .stdout(predicate::str::contains("disabled={false}"));
}