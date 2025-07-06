
#![allow(dead_code)] // a-llow dead code for this common helper module

use std::fs;
use std::io::Write;
use tempfile::TempDir;
use tempfile::tempdir;

/// A helper to set up a temporary directory with a multi-language sample project.
pub fn setup_test_project() -> TempDir {
    let dir = tempdir().unwrap();
    let src_dir = dir.path().join("src");
    fs::create_dir(&src_dir).unwrap();

    let main_rs_content = r#"
// TODO: Refactor this later
use crate::lib::User;

struct Cli {
    pattern: String,
}

pub fn main() {
    println!("Hello, world!");
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

def run_helper():
    h = Helper()
    return h.path
"#;
    let mut py_file = fs::File::create(dir.path().join("helper.py")).unwrap();
    py_file.write_all(py_content.as_bytes()).unwrap();

    // --- NEW: Add JS and TS files ---
    let js_content = r#"
// HACK: for demo purposes
import { a } from './lib';

export class OldLogger {
    log(msg) { console.log("logging: " + msg); }
}
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
    return { message };
}
"#;
    fs::File::create(src_dir.join("log_utils.ts"))
        .unwrap()
        .write_all(ts_content.as_bytes())
        .unwrap();

    dir
}