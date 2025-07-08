pub mod code_aware;
pub mod contains;
pub mod ext;
mod helpers;
pub mod in_path;
pub mod matches;
pub mod modified;
pub mod name;
pub mod path;
pub mod size;

use self::code_aware::CodeAwareEvaluator;
use self::contains::ContainsEvaluator;
use self::ext::ExtEvaluator;
use self::in_path::InPathEvaluator;
use self::matches::MatchesEvaluator;
use self::modified::ModifiedEvaluator;
use self::name::NameEvaluator;
use self::path::PathEvaluator;
use self::size::SizeEvaluator;
use crate::evaluator::{FileContext, MatchResult};
use crate::parser::PredicateKey;
use anyhow::Result;
use std::collections::HashMap;

// The core trait that all predicate evaluators must implement.
pub trait PredicateEvaluator {
    // The key is now passed to allow one evaluator to handle multiple predicate types.
    fn evaluate(
        &self,
        context: &mut FileContext,
        key: &PredicateKey,
        value: &str,
    ) -> Result<MatchResult>;
}

/// Creates a predicate registry with only the fast, metadata-based predicates.
/// This is used for the pre-filtering pass.
pub fn create_metadata_predicate_registry(
) -> HashMap<PredicateKey, Box<dyn PredicateEvaluator + Send + Sync>> {
    let mut registry: HashMap<PredicateKey, Box<dyn PredicateEvaluator + Send + Sync>> =
        HashMap::new();

    registry.insert(PredicateKey::Ext, Box::new(ExtEvaluator));
    registry.insert(PredicateKey::Name, Box::new(NameEvaluator));
    registry.insert(PredicateKey::Path, Box::new(PathEvaluator));
    registry.insert(PredicateKey::In, Box::new(InPathEvaluator));
    registry.insert(PredicateKey::Size, Box::new(SizeEvaluator));
    registry.insert(PredicateKey::Modified, Box::new(ModifiedEvaluator));

    registry
}

/// Creates and populates the complete predicate registry.
pub fn create_predicate_registry(
) -> HashMap<PredicateKey, Box<dyn PredicateEvaluator + Send + Sync>> {
    // Start with the metadata predicates
    let mut registry = create_metadata_predicate_registry();

    // Add content-based predicates
    registry.insert(PredicateKey::Contains, Box::new(ContainsEvaluator));
    registry.insert(PredicateKey::Matches, Box::new(MatchesEvaluator));

    // Register the single CodeAwareEvaluator for all semantic predicate keys.
    let code_evaluator = Box::new(CodeAwareEvaluator);
    registry.insert(PredicateKey::Def, code_evaluator.clone());
    registry.insert(PredicateKey::Func, code_evaluator.clone());
    registry.insert(PredicateKey::Import, code_evaluator.clone());
    registry.insert(PredicateKey::Class, code_evaluator.clone());
    registry.insert(PredicateKey::Struct, code_evaluator.clone());
    registry.insert(PredicateKey::Enum, code_evaluator.clone());
    registry.insert(PredicateKey::Interface, code_evaluator.clone());
    registry.insert(PredicateKey::Trait, code_evaluator.clone());
    registry.insert(PredicateKey::Type, code_evaluator.clone());
    registry.insert(PredicateKey::Comment, code_evaluator.clone());
    registry.insert(PredicateKey::Str, code_evaluator.clone());
    registry.insert(PredicateKey::Call, code_evaluator);

    registry
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    // The `code_aware` suite remains here as it tests the interaction
    // of multiple profiles, which is a responsibility of this parent module.
    #[test]
    fn test_code_aware_evaluator_full_rust_suite() {
        let rust_code = r#"
            // TODO: refactor this module
            use std::collections::HashMap;

            type ConfigMap = HashMap<String, String>;

            pub struct AppConfig {}
            pub trait Runnable {
                fn run(&self);
            }
            fn launch_app() {
                let msg = "Launching...";
                println!("{}", msg);
            }
        "#;

        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("complex.rs");
        let mut file = std::fs::File::create(&file_path).unwrap();
        file.write_all(rust_code.as_bytes()).unwrap();

        let evaluator = CodeAwareEvaluator;

        // --- Granular Defs ---
        let mut ctx = FileContext::new(file_path.clone(), file_path.parent().unwrap().to_path_buf());
        assert!(evaluator
            .evaluate(&mut ctx, &PredicateKey::Struct, "AppConfig")
            .unwrap()
            .is_match());
        let mut ctx = FileContext::new(file_path.clone(), file_path.parent().unwrap().to_path_buf());
        assert!(evaluator
            .evaluate(&mut ctx, &PredicateKey::Trait, "Runnable")
            .unwrap()
            .is_match());
        let mut ctx = FileContext::new(file_path.clone(), file_path.parent().unwrap().to_path_buf());
        assert!(evaluator
            .evaluate(&mut ctx, &PredicateKey::Type, "ConfigMap")
            .unwrap()
            .is_match());

        // --- Functions ---
        let mut ctx = FileContext::new(file_path.clone(), file_path.parent().unwrap().to_path_buf());
        assert!(evaluator
            .evaluate(&mut ctx, &PredicateKey::Func, "run")
            .unwrap()
            .is_match());
        let mut ctx = FileContext::new(file_path.clone(), file_path.parent().unwrap().to_path_buf());
        assert!(evaluator
            .evaluate(&mut ctx, &PredicateKey::Func, "launch_app")
            .unwrap()
            .is_match());

        // --- Calls ---
        let mut ctx = FileContext::new(file_path.clone(), file_path.parent().unwrap().to_path_buf());
        assert!(
            evaluator
                .evaluate(&mut ctx, &PredicateKey::Call, "println")
                .unwrap()
                .is_match(),
            "Should find function call"
        );
        let mut ctx = FileContext::new(file_path.clone(), file_path.parent().unwrap().to_path_buf());
        assert!(
            !evaluator
                .evaluate(&mut ctx, &PredicateKey::Call, "launch_app")
                .unwrap()
                .is_match(),
            "Should not find the definition as a call"
        );

        // --- Syntactic Content ---
        let mut ctx = FileContext::new(file_path.clone(), file_path.parent().unwrap().to_path_buf());
        assert!(evaluator
            .evaluate(&mut ctx, &PredicateKey::Comment, "TODO")
            .unwrap()
            .is_match());
        let mut ctx = FileContext::new(file_path.clone(), file_path.parent().unwrap().to_path_buf());
        assert!(evaluator
            .evaluate(&mut ctx, &PredicateKey::Str, "Launching...")
            .unwrap()
            .is_match());
    }

    #[test]
    fn test_code_aware_evaluator_python_suite() {
        let python_code = r#"
# FIXME: use a real database
import os

class DataProcessor:
    def __init__(self):
        self.api_key = "secret_key"
        self.connect()

    def connect(self):
        print("Connecting...")

def process_data():
    proc = DataProcessor()
    print("Processing")
        "#;

        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("script.py");
        let mut file = std::fs::File::create(&file_path).unwrap();
        file.write_all(python_code.as_bytes()).unwrap();

        let evaluator = CodeAwareEvaluator;

        // --- Granular Defs ---
        let mut ctx = FileContext::new(file_path.clone(), file_path.parent().unwrap().to_path_buf());
        assert!(evaluator
            .evaluate(&mut ctx, &PredicateKey::Class, "DataProcessor")
            .unwrap()
            .is_match());

        // --- Functions ---
        let mut ctx = FileContext::new(file_path.clone(), file_path.parent().unwrap().to_path_buf());
        assert!(evaluator
            .evaluate(&mut ctx, &PredicateKey::Func, "process_data")
            .unwrap()
            .is_match());
        let mut ctx = FileContext::new(file_path.clone(), file_path.parent().unwrap().to_path_buf());
        assert!(evaluator
            .evaluate(&mut ctx, &PredicateKey::Func, "connect")
            .unwrap()
            .is_match());

        // --- Calls ---
        let mut ctx = FileContext::new(file_path.clone(), file_path.parent().unwrap().to_path_buf());
        assert!(
            evaluator
                .evaluate(&mut ctx, &PredicateKey::Call, "print")
                .unwrap()
                .is_match(),
            "Should find multiple calls to print"
        );
        let mut ctx = FileContext::new(file_path.clone(), file_path.parent().unwrap().to_path_buf());
        assert!(
            evaluator
                .evaluate(&mut ctx, &PredicateKey::Call, "DataProcessor")
                .unwrap()
                .is_match(),
            "Should find constructor call"
        );
        let mut ctx = FileContext::new(file_path.clone(), file_path.parent().unwrap().to_path_buf());
        assert!(
            evaluator
                .evaluate(&mut ctx, &PredicateKey::Call, "connect")
                .unwrap()
                .is_match(),
            "Should find method call"
        );

        // --- Syntactic Content ---
        let mut ctx = FileContext::new(file_path.clone(), file_path.parent().unwrap().to_path_buf());
        assert!(evaluator
            .evaluate(&mut ctx, &PredicateKey::Comment, "FIXME")
            .unwrap()
            .is_match());
        let mut ctx = FileContext::new(file_path.clone(), file_path.parent().unwrap().to_path_buf());
        assert!(evaluator
            .evaluate(&mut ctx, &PredicateKey::Str, "secret_key")
            .unwrap()
            .is_match());
    }

    #[test]
    fn test_code_aware_evaluator_javascript_suite() {
        let js_code = r#"
            import { open } from 'fs/promises';

            class Logger {
                log(message) { console.log(message); }
            }

            function a() {
                const l = new Logger();
                l.log("hello");
            }
        "#;

        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("script.js");
        let mut file = std::fs::File::create(&file_path).unwrap();
        file.write_all(js_code.as_bytes()).unwrap();

        let evaluator = CodeAwareEvaluator;

        let mut ctx = FileContext::new(file_path.clone(), file_path.parent().unwrap().to_path_buf());
        assert!(evaluator
            .evaluate(&mut ctx, &PredicateKey::Def, "Logger")
            .unwrap()
            .is_match());
        let mut ctx = FileContext::new(file_path.clone(), file_path.parent().unwrap().to_path_buf());
        assert!(evaluator
            .evaluate(&mut ctx, &PredicateKey::Func, "log")
            .unwrap()
            .is_match());
        let mut ctx = FileContext::new(file_path.clone(), file_path.parent().unwrap().to_path_buf());
        assert!(evaluator
            .evaluate(&mut ctx, &PredicateKey::Import, "fs/promises")
            .unwrap()
            .is_match());
        let mut ctx = FileContext::new(file_path.clone(), file_path.parent().unwrap().to_path_buf());
        assert!(
            evaluator
                .evaluate(&mut ctx, &PredicateKey::Call, "Logger")
                .unwrap()
                .is_match(),
            "Should find constructor call"
        );
        let mut ctx = FileContext::new(file_path.clone(), file_path.parent().unwrap().to_path_buf());
        assert!(
            evaluator
                .evaluate(&mut ctx, &PredicateKey::Call, "log")
                .unwrap()
                .is_match(),
            "Should find method call"
        );
    }

    #[test]
    fn test_code_aware_evaluator_typescript_suite() {
        let ts_code = r#"
            import React from 'react';

            interface User { id: number; }
            type ID = string | number;

            class ApiClient {
                // The URL for the API
                private url = "https://api.example.com";
                fetchUser(): User | null { return null; }
            }

            const client = new ApiClient();
            client.fetchUser();
        "#;

        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("api.ts");
        let mut file = std::fs::File::create(&file_path).unwrap();
        file.write_all(ts_code.as_bytes()).unwrap();

        let evaluator = CodeAwareEvaluator;

        // --- Granular Defs ---
        let mut ctx = FileContext::new(file_path.clone(), file_path.parent().unwrap().to_path_buf());
        assert!(
            evaluator
                .evaluate(&mut ctx, &PredicateKey::Def, "ApiClient")
                .unwrap()
                .is_match(),
            "Should find class"
        );
        let mut ctx = FileContext::new(file_path.clone(), file_path.parent().unwrap().to_path_buf());
        assert!(evaluator
            .evaluate(&mut ctx, &PredicateKey::Func, "fetchUser")
            .unwrap()
            .is_match());
        let mut ctx = FileContext::new(file_path.clone(), file_path.parent().unwrap().to_path_buf());
        assert!(evaluator
            .evaluate(&mut ctx, &PredicateKey::Import, "React")
            .unwrap()
            .is_match());
        let mut ctx = FileContext::new(file_path.clone(), file_path.parent().unwrap().to_path_buf());
        assert!(
            evaluator
                .evaluate(&mut ctx, &PredicateKey::Call, "ApiClient")
                .unwrap()
                .is_match(),
            "Should find TS constructor call"
        );
        let mut ctx = FileContext::new(file_path.clone(), file_path.parent().unwrap().to_path_buf());
        assert!(
            evaluator
                .evaluate(&mut ctx, &PredicateKey::Call, "fetchUser")
                .unwrap()
                .is_match(),
            "Should find TS method call"
        );

        // --- Syntactic Content ---
        let mut ctx = FileContext::new(file_path.clone(), file_path.parent().unwrap().to_path_buf());
        assert!(evaluator
            .evaluate(&mut ctx, &PredicateKey::Comment, "The URL")
            .unwrap()
            .is_match());
        let mut ctx = FileContext::new(file_path.clone(), file_path.parent().unwrap().to_path_buf());
        assert!(evaluator
            .evaluate(&mut ctx, &PredicateKey::Str, "https://api.example.com")
            .unwrap()
            .is_match());
    }

    #[test]
    fn test_code_aware_evaluator_go_suite() {
        let go_code = r#"
           package main

           import "fmt"

           // User represents a user
           type User struct {
               ID int
           }

           func (u *User) Greet() {
               fmt.Println("Hello")
           }

           func main() {
               user := User{ID: 1}
               user.Greet()
           }
       "#;

        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("main.go");
        let mut file = std::fs::File::create(&file_path).unwrap();
        file.write_all(go_code.as_bytes()).unwrap();

        let evaluator = CodeAwareEvaluator;

        let mut ctx = FileContext::new(file_path.clone(), file_path.parent().unwrap().to_path_buf());
        assert!(evaluator
            .evaluate(&mut ctx, &PredicateKey::Struct, "User")
            .unwrap()
            .is_match());
        let mut ctx = FileContext::new(file_path.clone(), file_path.parent().unwrap().to_path_buf());
        assert!(evaluator
            .evaluate(&mut ctx, &PredicateKey::Func, "Greet")
            .unwrap()
            .is_match());
        let mut ctx = FileContext::new(file_path.clone(), file_path.parent().unwrap().to_path_buf());
        assert!(evaluator
            .evaluate(&mut ctx, &PredicateKey::Call, "Println")
            .unwrap()
            .is_match());
        let mut ctx = FileContext::new(file_path.clone(), file_path.parent().unwrap().to_path_buf());
        assert!(evaluator
            .evaluate(&mut ctx, &PredicateKey::Import, "fmt")
            .unwrap()
            .is_match());
        let mut ctx = FileContext::new(file_path.clone(), file_path.parent().unwrap().to_path_buf());
        assert!(evaluator
            .evaluate(&mut ctx, &PredicateKey::Comment, "represents a user")
            .unwrap()
            .is_match());
    }

    #[test]
    fn test_code_aware_evaluator_java_suite() {
        let java_code = r#"
           package com.example;

           import java.util.List;

           // Represents a user
           public class User {
               public User() {
                   System.out.println("User created");
               }

               public void greet() {}
           }
       "#;

        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("User.java");
        let mut file = std::fs::File::create(&file_path).unwrap();
        file.write_all(java_code.as_bytes()).unwrap();

        let evaluator = CodeAwareEvaluator;

        let mut ctx = FileContext::new(file_path.clone(), file_path.parent().unwrap().to_path_buf());
        assert!(evaluator
            .evaluate(&mut ctx, &PredicateKey::Class, "User")
            .unwrap()
            .is_match());
        let mut ctx = FileContext::new(file_path.clone(), file_path.parent().unwrap().to_path_buf());
        assert!(evaluator
            .evaluate(&mut ctx, &PredicateKey::Func, "greet")
            .unwrap()
            .is_match());
        let mut ctx = FileContext::new(file_path.clone(), file_path.parent().unwrap().to_path_buf());
        assert!(evaluator
            .evaluate(&mut ctx, &PredicateKey::Call, "println")
            .unwrap()
            .is_match());
        let mut ctx = FileContext::new(file_path.clone(), file_path.parent().unwrap().to_path_buf());
        assert!(evaluator
            .evaluate(&mut ctx, &PredicateKey::Import, "java.util.List")
            .unwrap()
            .is_match());
        let mut ctx = FileContext::new(file_path.clone(), file_path.parent().unwrap().to_path_buf());
        assert!(evaluator
            .evaluate(&mut ctx, &PredicateKey::Comment, "Represents a user")
            .unwrap()
            .is_match());
        let mut ctx = FileContext::new(file_path.clone(), file_path.parent().unwrap().to_path_buf());
        assert!(evaluator
            .evaluate(&mut ctx, &PredicateKey::Str, "User created")
            .unwrap()
            .is_match());
    }
}
