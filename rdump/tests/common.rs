#![allow(dead_code)] // a-llow dead code for this common helper module

use std::fs;
use std::io::Write;
use tempfile::tempdir;
use tempfile::TempDir;

/// A helper to set up a temporary directory with a multi-language sample project.
pub fn setup_test_project() -> TempDir {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    fs::create_dir(&src_dir).unwrap();

    let main_rs_content = r#"
#[macro_use]
mod macros;
mod lib;
mod traits;

// TODO: Refactor this later
use crate::lib::{User, Role};

struct Cli {
    pattern: String,
}

impl Cli {
    fn new() -> Self { Self { pattern: "".into() } }
}

pub fn main() {
    // This is the main function
    let _u = User::new();
    println!("Hello, world!");
    my_macro!();
}
"#;
    let mut main_rs = fs::File::create(src_dir.join("main.rs")).unwrap();
    main_rs.write_all(main_rs_content.as_bytes()).unwrap();

    let lib_rs_content = r#"
// This is a library file.
use serde::Serialize;

pub type UserId = u64;

pub struct User {
    id: UserId,
    name: String,
}

impl User {
    pub fn new() -> Self {
        Self { id: 0, name: "".into() }
    }
}

pub enum Role {
    Admin,
    User,
}
"#;
    let mut lib_rs = fs::File::create(src_dir.join("lib.rs")).unwrap();
    lib_rs.write_all(lib_rs_content.as_bytes()).unwrap();

    let readme_md_content = "# Test Project\nThis is a README for Role and User structs.";
    let mut readme_md = fs::File::create(dir.path().join("README.md")).unwrap();
    readme_md.write_all(readme_md_content.as_bytes()).unwrap();

    // --- Add a Python file ---
    let py_content = r#"
# FIXME: Hardcoded path
import os

class Helper:
    def __init__(self):
        self.path = "/tmp/data"
        self.do_setup()

    def do_setup(self):
        print("Setup complete")

def run_helper():
    h = Helper()
    return h.path

if __name__ == "__main__":
    run_helper()
"#;
    let mut py_file = fs::File::create(dir.path().join("helper.py")).unwrap();
    py_file.write_all(py_content.as_bytes()).unwrap();

    // --- Add JS and TS files ---
    let js_content = r#"
// HACK: for demo purposes
import { a } from './lib';

export class OldLogger {
    log(msg) { console.log("logging: " + msg); }
}

const logger = new OldLogger();
logger.log("init");
"#;
    fs::File::create(src_dir.join("logger.js"))
        .unwrap()
        .write_all(js_content.as_bytes())
        .unwrap();

    let ts_content = r#"
// REVIEW: Use a real logging library
import * as path from 'path';

export interface ILog {
    message: string;
}

export type LogLevel = "info" | "warn" | "error";

export function createLog(message: string): ILog {
    const newLog = { message };
    console.log(newLog);
    return newLog;
}
"#;
    fs::File::create(src_dir.join("log_utils.ts"))
        .unwrap()
        .write_all(ts_content.as_bytes())
        .unwrap();

    // --- Add a Go file ---
    let go_content = r#"
package main

import "fmt"

// Server represents our HTTP server.
type Server struct {
	Address string
}

func NewServer(addr string) *Server {
	return &Server{Address: addr}
}

func main() {
	server := NewServer(":8080")
	fmt.Println(server.Address)
}
"#;
    fs::File::create(src_dir.join("main.go"))
        .unwrap()
        .write_all(go_content.as_bytes())
        .unwrap();

    // --- Add a Java file ---
    let java_dir = dir.path().join("src/main/java/com/example");
    fs::create_dir_all(&java_dir).unwrap();
    let java_content = r#"
package com.example;

import java.util.ArrayList;

/**
 * Main application class.
 * HACK: This is just for a test.
 */
public class Application {
    public static void main(String[] args) {
        ArrayList<String> list = new ArrayList<>();
        System.out.println("Hello from Java!");
    }
}
"#;
    fs::File::create(java_dir.join("Application.java"))
        .unwrap()
        .write_all(java_content.as_bytes())
        .unwrap();

    let traits_rs_content = r#"
pub trait Summary {
    fn summarize(&self) -> String;
}

pub struct NewsArticle {
    pub headline: String,
    pub location: String,
    pub author: String,
    pub content: String,
}

impl Summary for NewsArticle {
    fn summarize(&self) -> String {
        format!("{}, by {} ({})", self.headline, self.author, self.location)
    }
}
"#;
    let mut traits_rs = fs::File::create(src_dir.join("traits.rs")).unwrap();
    traits_rs.write_all(traits_rs_content.as_bytes()).unwrap();

    let macros_rs_content = r#"
#[macro_export]
macro_rules! my_macro {
    () => {
        println!("This is my macro!");
    };
}
"#;
    let mut macros_rs = fs::File::create(src_dir.join("macros.rs")).unwrap();
    macros_rs.write_all(macros_rs_content.as_bytes()).unwrap();

    // --- Add React Test Files ---
    let app_tsx_content = r#"
import React, { useState } from 'react';
import { Button } from './Button';
import useAuth from './useAuth';

// A simple component
function App() {
  const [count, setCount] = useState(0);
  const { user } = useAuth();

  return (
    <div>
      <h1>Welcome, {user?.name}</h1>
      <p>Count: {count}</p>
      <Button onClick={() => setCount(c => c + 1)} disabled={false} />
    </div>
  );
}
export default App;
"#;
    fs::File::create(src_dir.join("App.tsx"))
        .unwrap()
        .write_all(app_tsx_content.as_bytes())
        .unwrap();

    let button_jsx_content = r#"
// A button component
export const Button = ({ onClick, disabled }) => {
  return <button onClick={onClick} disabled={disabled}>Click Me</button>;
};
"#;
    fs::File::create(src_dir.join("Button.jsx"))
        .unwrap()
        .write_all(button_jsx_content.as_bytes())
        .unwrap();

    let hook_ts_content = r#"
// A custom hook
export default function useAuth() {
  return { user: { name: 'Guest' } };
}
"#;
    fs::File::create(src_dir.join("useAuth.ts"))
        .unwrap()
        .write_all(hook_ts_content.as_bytes())
        .unwrap();

    dir
}