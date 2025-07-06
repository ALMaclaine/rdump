
pub mod code_aware;
pub mod contains;
pub mod ext;
mod helpers;
pub mod matches;
pub mod modified;
pub mod name;
pub mod path;
pub mod size;

use self::code_aware::CodeAwareEvaluator;
use self::contains::ContainsEvaluator;
use self::ext::ExtEvaluator;
use self::matches::MatchesEvaluator;
use self::modified::ModifiedEvaluator;
use self::name::NameEvaluator;
use self::path::PathEvaluator;
use self::size::SizeEvaluator;
use crate::evaluator::FileContext;
use crate::parser::PredicateKey;
use anyhow::Result;
use std::collections::HashMap;

// The core trait that all predicate evaluators must implement.
pub trait PredicateEvaluator {
    // The key is now passed to allow one evaluator to handle multiple predicate types.
    fn evaluate(&self, context: &mut FileContext, key: &PredicateKey, value: &str) -> Result<bool>;
}

/// Creates and populates the predicate registry.
pub fn create_predicate_registry(
) -> HashMap<PredicateKey, Box<dyn PredicateEvaluator + Send + Sync>> {
    let mut registry: HashMap<PredicateKey, Box<dyn PredicateEvaluator + Send + Sync>> =
        HashMap::new();

    registry.insert(PredicateKey::Ext, Box::new(ExtEvaluator));
    registry.insert(PredicateKey::Name, Box::new(NameEvaluator));
    registry.insert(PredicateKey::Path, Box::new(PathEvaluator));
    registry.insert(PredicateKey::Contains, Box::new(ContainsEvaluator));
    registry.insert(PredicateKey::Matches, Box::new(MatchesEvaluator));
    registry.insert(PredicateKey::Size, Box::new(SizeEvaluator));
    registry.insert(PredicateKey::Modified, Box::new(ModifiedEvaluator));

    // Register the single CodeAwareEvaluator for all semantic predicate keys.
    // It's a stateless struct, so cloning the Box is cheap (it's just a pointer clone).
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
// ... (basic predicate tests are unchanged) ...
    use super::*;
    use std::io::Write;
    use std::path::PathBuf;
    use tempfile::NamedTempFile;

    fn create_temp_file(content: &str) -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "{}", content).unwrap();
        file
    }

    #[test]
    fn test_size_evaluator() {
        let file = create_temp_file("a".repeat(2000).as_str());
        let mut context = FileContext::new(file.path().to_path_buf());

        let evaluator = SizeEvaluator;
        assert!(evaluator
            .evaluate(&mut context, &PredicateKey::Size, ">1000")
            .unwrap());
        assert!(!evaluator
            .evaluate(&mut context, &PredicateKey::Size, "<1kb")
            .unwrap());
        assert!(evaluator
            .evaluate(&mut context, &PredicateKey::Size, ">0.9kb")
            .unwrap());
    }

    #[test]
    fn test_modified_evaluator() {
        let file = create_temp_file("content");
        let mut context = FileContext::new(file.path().to_path_buf());

        let evaluator = ModifiedEvaluator;
        // File was just created
        assert!(evaluator
            .evaluate(&mut context, &PredicateKey::Modified, ">1m")
            .unwrap()); // Modified more recently than 1 min ago
        assert!(!evaluator
            .evaluate(&mut context, &PredicateKey::Modified, "<1m")
            .unwrap()); // Not modified longer than 1 min ago
    }

    #[test]
    fn test_ext_evaluator() {
        let mut context_rs = FileContext::new(PathBuf::from("main.rs"));
        let mut context_toml = FileContext::new(PathBuf::from("Cargo.TOML"));
        let mut context_no_ext = FileContext::new(PathBuf::from("README"));
        let mut context_dotfile = FileContext::new(PathBuf::from(".bashrc"));

        let evaluator = ExtEvaluator;
        assert!(evaluator
            .evaluate(&mut context_rs, &PredicateKey::Ext, "rs")
            .unwrap());
        assert!(!evaluator
            .evaluate(&mut context_rs, &PredicateKey::Ext, "toml")
            .unwrap());
        assert!(
            evaluator
                .evaluate(&mut context_toml, &PredicateKey::Ext, "toml")
                .unwrap(),
            "Should be case-insensitive"
        );
        assert!(!evaluator
            .evaluate(&mut context_no_ext, &PredicateKey::Ext, "rs")
            .unwrap());
        assert!(
            !evaluator
                .evaluate(&mut context_dotfile, &PredicateKey::Ext, "bashrc")
                .unwrap(),
            "Dotfiles should have no extension"
        );
    }

    #[test]
    fn test_path_evaluator() {
        let mut context = FileContext::new(PathBuf::from("/home/user/project/src/main.rs"));
        let evaluator = PathEvaluator;
        assert!(evaluator
            .evaluate(&mut context, &PredicateKey::Path, "project/src")
            .unwrap());
        assert!(evaluator
            .evaluate(&mut context, &PredicateKey::Path, "/home/user")
            .unwrap());
        assert!(!evaluator
            .evaluate(&mut context, &PredicateKey::Path, "project/lib")
            .unwrap());
        assert!(evaluator
            .evaluate(&mut context, &PredicateKey::Path, "main.rs")
            .unwrap());
    }

    #[test]
    fn test_name_evaluator() {
        let mut context1 = FileContext::new(PathBuf::from("/home/user/Cargo.toml"));
        let mut context2 = FileContext::new(PathBuf::from("/home/user/main.rs"));

        let evaluator = NameEvaluator;
        assert!(evaluator
            .evaluate(&mut context1, &PredicateKey::Name, "Cargo.toml")
            .unwrap());
        assert!(
            evaluator
                .evaluate(&mut context1, &PredicateKey::Name, "C*.toml")
                .unwrap(),
            "Glob pattern should match"
        );
        assert!(
            evaluator
                .evaluate(&mut context2, &PredicateKey::Name, "*.rs")
                .unwrap(),
            "Glob pattern should match"
        );
        assert!(!evaluator
            .evaluate(&mut context1, &PredicateKey::Name, "*.rs")
            .unwrap());
    }

    #[test]
    fn test_contains_evaluator() {
        let file = create_temp_file("Hello world\nThis is a test.");
        let mut context = FileContext::new(file.path().to_path_buf());
        let evaluator = ContainsEvaluator;
        assert!(evaluator
            .evaluate(&mut context, &PredicateKey::Contains, "world")
            .unwrap());
        assert!(evaluator
            .evaluate(&mut context, &PredicateKey::Contains, "is a test")
            .unwrap());
        assert!(!evaluator
            .evaluate(&mut context, &PredicateKey::Contains, "goodbye")
            .unwrap());
    }

    #[test]
    fn test_matches_evaluator() {
        let file = create_temp_file("version = \"0.1.0\"\nauthor = \"test\"");
        let mut context = FileContext::new(file.path().to_path_buf());
        let evaluator = MatchesEvaluator;
        // Simple regex
        assert!(evaluator
            .evaluate(
                &mut context,
                &PredicateKey::Matches,
                "version = \"[0-9]+\\.[0-9]+\\.[0-9]+\""
            )
            .unwrap());
        // Test regex that spans lines
        assert!(evaluator
            .evaluate(&mut context, &PredicateKey::Matches, "(?s)version.*author")
            .unwrap());
        assert!(!evaluator
            .evaluate(
                &mut context,
                &PredicateKey::Matches,
                "^version = \"1.0.0\"$"
            )
            .unwrap());
    }

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
        let mut ctx = FileContext::new(file_path.clone());
        assert!(evaluator.evaluate(&mut ctx, &PredicateKey::Struct, "AppConfig").unwrap());
        let mut ctx = FileContext::new(file_path.clone());
        assert!(evaluator.evaluate(&mut ctx, &PredicateKey::Trait, "Runnable").unwrap());
        let mut ctx = FileContext::new(file_path.clone());
        assert!(evaluator.evaluate(&mut ctx, &PredicateKey::Type, "ConfigMap").unwrap());

        // --- Functions ---
        let mut ctx = FileContext::new(file_path.clone());
        assert!(evaluator.evaluate(&mut ctx, &PredicateKey::Func, "run").unwrap());
        let mut ctx = FileContext::new(file_path.clone());
        assert!(evaluator.evaluate(&mut ctx, &PredicateKey::Func, "launch_app").unwrap());

       // --- Calls ---
       let mut ctx = FileContext::new(file_path.clone());
       assert!(evaluator.evaluate(&mut ctx, &PredicateKey::Call, "println").unwrap(), "Should find function call");
       let mut ctx = FileContext::new(file_path.clone());
       assert!(!evaluator.evaluate(&mut ctx, &PredicateKey::Call, "launch_app").unwrap(), "Should not find the definition as a call");

        // --- Syntactic Content ---
        let mut ctx = FileContext::new(file_path.clone());
        assert!(evaluator.evaluate(&mut ctx, &PredicateKey::Comment, "TODO").unwrap());
        let mut ctx = FileContext::new(file_path.clone());
        assert!(evaluator.evaluate(&mut ctx, &PredicateKey::Str, "Launching...").unwrap());
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
        let mut ctx = FileContext::new(file_path.clone());
        assert!(evaluator.evaluate(&mut ctx, &PredicateKey::Class, "DataProcessor").unwrap());

        // --- Functions ---
        let mut ctx = FileContext::new(file_path.clone());
        assert!(evaluator.evaluate(&mut ctx, &PredicateKey::Func, "process_data").unwrap());
        let mut ctx = FileContext::new(file_path.clone());
        assert!(evaluator.evaluate(&mut ctx, &PredicateKey::Func, "connect").unwrap());

       // --- Calls ---
       let mut ctx = FileContext::new(file_path.clone());
       assert!(evaluator.evaluate(&mut ctx, &PredicateKey::Call, "print").unwrap(), "Should find multiple calls to print");
       let mut ctx = FileContext::new(file_path.clone());
       assert!(evaluator.evaluate(&mut ctx, &PredicateKey::Call, "DataProcessor").unwrap(), "Should find constructor call");
       let mut ctx = FileContext::new(file_path.clone());
       assert!(evaluator.evaluate(&mut ctx, &PredicateKey::Call, "connect").unwrap(), "Should find method call");

        // --- Syntactic Content ---
        let mut ctx = FileContext::new(file_path.clone());
        assert!(evaluator.evaluate(&mut ctx, &PredicateKey::Comment, "FIXME").unwrap());
        let mut ctx = FileContext::new(file_path.clone());
        assert!(evaluator.evaluate(&mut ctx, &PredicateKey::Str, "secret_key").unwrap());
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

        let mut ctx = FileContext::new(file_path.clone());
        assert!(evaluator.evaluate(&mut ctx, &PredicateKey::Def, "Logger").unwrap());
        let mut ctx = FileContext::new(file_path.clone());
        assert!(evaluator.evaluate(&mut ctx, &PredicateKey::Func, "log").unwrap());
        let mut ctx = FileContext::new(file_path.clone());
        assert!(evaluator.evaluate(&mut ctx, &PredicateKey::Import, "fs/promises").unwrap());
       let mut ctx = FileContext::new(file_path.clone());
       assert!(evaluator.evaluate(&mut ctx, &PredicateKey::Call, "Logger").unwrap(), "Should find constructor call");
       let mut ctx = FileContext::new(file_path.clone());
       assert!(evaluator.evaluate(&mut ctx, &PredicateKey::Call, "log").unwrap(), "Should find method call");
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
        let mut ctx = FileContext::new(file_path.clone());
        assert!(evaluator.evaluate(&mut ctx, &PredicateKey::Def, "ApiClient").unwrap(), "Should find class");
        let mut ctx = FileContext::new(file_path.clone());
        assert!(evaluator.evaluate(&mut ctx, &PredicateKey::Func, "fetchUser").unwrap());
        let mut ctx = FileContext::new(file_path.clone());
        assert!(evaluator.evaluate(&mut ctx, &PredicateKey::Import, "React").unwrap());
       let mut ctx = FileContext::new(file_path.clone());
       assert!(evaluator.evaluate(&mut ctx, &PredicateKey::Call, "ApiClient").unwrap(), "Should find TS constructor call");
       let mut ctx = FileContext::new(file_path.clone());
       assert!(evaluator.evaluate(&mut ctx, &PredicateKey::Call, "fetchUser").unwrap(), "Should find TS method call");

        // --- Syntactic Content ---
        let mut ctx = FileContext::new(file_path.clone());
        assert!(evaluator.evaluate(&mut ctx, &PredicateKey::Comment, "The URL").unwrap());
        let mut ctx = FileContext::new(file_path.clone());
        assert!(evaluator.evaluate(&mut ctx, &PredicateKey::Str, "https://api.example.com").unwrap());
    }
}
