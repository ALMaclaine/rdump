use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::fs;
use std::io::Write;
use std::process::Command;
use tempfile::TempDir;

mod common;

fn setup_react_test_project() -> TempDir {
    let dir = common::setup_test_project();
    let src_dir = dir.path().join("src");

    let jsx_content = r#"
import React from 'react';

function MyComponent({ prop1 }) {
  return <div className="my-component">Hello, World!</div>;
}

const AnotherComponent = () => {
  return <MyComponent prop1="test" />;
};

export default AnotherComponent;
"#;
    fs::File::create(src_dir.join("test.jsx"))
        .unwrap()
        .write_all(jsx_content.as_bytes())
        .unwrap();

    let tsx_content = r#"
import React, { useState, useEffect } from 'react';

// A custom hook
function useData(url: string) {
  const [data, setData] = useState(null);
  useEffect(() => {
    fetch(url).then(res => res.json()).then(setData);
  }, [url]);
  return data;
}

interface MyComponentProps {
  prop2: string;
}

const MyTsxComponent: React.FC<MyComponentProps> = ({ prop2 }) => {
  const data = useData('/api/data');
  const [count, setCount] = useState(0);

  return (
    <div onClick={() => setCount(count + 1)}>
      <p>Data: {JSON.stringify(data)}</p>
      <p>Count: {count}</p>
    </div>
  );
};

export default MyTsxComponent;
"#;
    fs::File::create(src_dir.join("test.tsx"))
        .unwrap()
        .write_all(tsx_content.as_bytes())
        .unwrap();

    dir
}

#[test]
fn test_component_predicate() {
    let dir = setup_react_test_project();
    Command::cargo_bin("rdump")
        .unwrap()
        .current_dir(dir.path())
        .arg("search")
        .arg("component:MyComponent")
        .assert()
        .success()
        .stdout(predicate::str::contains("test.jsx"))
        .stdout(predicate::str::contains("function MyComponent({ prop1 })"));
}

#[test]
fn test_element_predicate() {
    let dir = setup_react_test_project();
    Command::cargo_bin("rdump")
        .unwrap()
        .current_dir(dir.path())
        .arg("search")
        .arg("element:div")
        .assert()
        .success()
        .stdout(predicate::str::contains("test.jsx"))
        .stdout(predicate::str::contains("test.tsx"));
}

#[test]
fn test_hook_predicate() {
    let dir = setup_react_test_project();
    Command::cargo_bin("rdump")
        .unwrap()
        .current_dir(dir.path())
        .arg("search")
        .arg("hook:useState")
        .assert()
        .success()
        .stdout(predicate::str::contains("test.tsx"))
        .stdout(predicate::str::contains("const [data, setData] = useState(null);"));
}

#[test]
fn test_custom_hook_predicate() {
    let dir = setup_react_test_project();
    Command::cargo_bin("rdump")
        .unwrap()
        .current_dir(dir.path())
        .arg("search")
        .arg("customhook:useData")
        .assert()
        .success()
        .stdout(predicate::str::contains("test.tsx"))
        .stdout(predicate::str::contains("function useData(url: string)"));
}

#[test]
fn test_prop_predicate() {
    let dir = setup_react_test_project();
    Command::cargo_bin("rdump")
        .unwrap()
        .current_dir(dir.path())
        .arg("search")
        .arg("prop:prop1")
        .assert()
        .success()
        .stdout(predicate::str::contains("test.jsx"))
        .stdout(predicate::str::contains("return <MyComponent prop1=\"test\" />;"));
}

#[test]
fn test_combined_react_query() {
    let dir = setup_react_test_project();
    Command::cargo_bin("rdump")
        .unwrap()
        .current_dir(dir.path())
        .arg("search")
        .arg("component:MyTsxComponent & element:p & hook:useState")
        .assert()
        .success()
        .stdout(predicate::str::contains("test.tsx"));
}
