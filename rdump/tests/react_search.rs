use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;

mod common;
use common::setup_test_project;

#[test]
fn test_component_predicate_finds_functional_component() {
    let dir = setup_test_project();
    Command::cargo_bin("rdump")
        .unwrap()
        .current_dir(dir.path())
        .arg("search")
        .arg("component:App & ext:tsx")
        .assert()
        .success()
        .stdout(predicate::str::contains("function App()"));
}

#[test]
fn test_component_predicate_finds_arrow_function_component() {
    let dir = setup_test_project();
    Command::cargo_bin("rdump")
        .unwrap()
        .current_dir(dir.path())
        .arg("search")
        .arg("component:Button & ext:jsx")
        .assert()
        .success()
        .stdout(predicate::str::contains("export const Button"));
}

#[test]
fn test_element_predicate_finds_html_element() {
    let dir = setup_test_project();
    Command::cargo_bin("rdump")
        .unwrap()
        .current_dir(dir.path())
        .arg("search")
        .arg("element:h1 & ext:tsx")
        .assert()
        .success()
        .stdout(predicate::str::contains("<h1>Welcome, {user?.name}</h1>"));
}

#[test]
fn test_element_predicate_finds_component_element() {
    let dir = setup_test_project();
    Command::cargo_bin("rdump")
        .unwrap()
        .current_dir(dir.path())
        .arg("search")
        .arg("element:Button & ext:tsx")
        .assert()
        .success()
        .stdout(predicate::str::contains("<Button onClick="));
}

#[test]
fn test_hook_predicate_finds_built_in_hook() {
    let dir = setup_test_project();
    Command::cargo_bin("rdump")
        .unwrap()
        .current_dir(dir.path())
        .arg("search")
        .arg("hook:useState")
        .assert()
        .success()
        .stdout(predicate::str::contains("const [count, setCount] = useState(0);"));
}

#[test]
fn test_hook_predicate_finds_custom_hook_call() {
    let dir = setup_test_project();
    Command::cargo_bin("rdump")
        .unwrap()
        .current_dir(dir.path())
        .arg("search")
        .arg("hook:useAuth")
        .assert()
        .success()
        .stdout(predicate::str::contains("const { user } = useAuth();"));
}

#[test]
fn test_customhook_predicate_finds_hook_definition() {
    let dir = setup_test_project();
    Command::cargo_bin("rdump")
        .unwrap()
        .current_dir(dir.path())
        .arg("search")
        .arg("customhook:useAuth")
        .assert()
        .success()
        .stdout(predicate::str::contains("export default function useAuth()"));
}

#[test]
fn test_prop_predicate_finds_prop() {
    let dir = setup_test_project();
    Command::cargo_bin("rdump")
        .unwrap()
        .current_dir(dir.path())
        .arg("search")
        .arg("prop:onClick")
        .assert()
        .success()
        .stdout(predicate::str::contains("<Button onClick={")) // In App.tsx
        .stdout(predicate::str::contains("<button onClick={onClick}")); // In Button.jsx
}

#[test]
fn test_react_and_logic_across_predicates() {
    let dir = setup_test_project();
    // Find a Button element that is also passed a `disabled` prop.
    Command::cargo_bin("rdump")
        .unwrap()
        .current_dir(dir.path())
        .arg("search")
        .arg("element:Button & prop:disabled")
        .assert()
        .success()
        .stdout(predicate::str::contains("App.tsx"))
        .stdout(predicate::str::contains("Button.jsx").not());
}