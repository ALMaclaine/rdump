pub mod code_aware;

use crate::evaluator::FileContext;
use crate::parser::PredicateKey;
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use self::code_aware::CodeAwareEvaluator;

// The core trait that all predicate evaluators must implement.
pub trait PredicateEvaluator {
    // The key is now passed to allow one evaluator to handle multiple predicate types.
    fn evaluate(&self, context: &mut FileContext, key: &PredicateKey, value: &str) -> Result<bool>;
}

// --- Concrete Implementations ---

struct ExtEvaluator;
impl PredicateEvaluator for ExtEvaluator {
    fn evaluate(&self, context: &mut FileContext, _key: &PredicateKey, value: &str) -> Result<bool> {
        let file_ext = context.path.extension().and_then(|s| s.to_str()).unwrap_or("");
        Ok(file_ext.eq_ignore_ascii_case(value))
    }
}

struct PathEvaluator;
impl PredicateEvaluator for PathEvaluator {
    fn evaluate(&self, context: &mut FileContext, _key: &PredicateKey, value: &str) -> Result<bool> {
        let path_str = context.path.to_string_lossy();
        Ok(path_str.contains(value))
    }
}

struct NameEvaluator;
impl PredicateEvaluator for NameEvaluator {
    fn evaluate(&self, context: &mut FileContext, _key: &PredicateKey, value: &str) -> Result<bool> {
        let file_name = context.path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        let pattern = glob::Pattern::new(value)?;
        Ok(pattern.matches(file_name))
    }
}

struct ContainsEvaluator;
impl PredicateEvaluator for ContainsEvaluator {
    fn evaluate(&self, context: &mut FileContext, _key: &PredicateKey, value: &str) -> Result<bool> {
        let content = context.get_content()?;
        Ok(content.contains(value))
    }
}

struct MatchesEvaluator;
impl PredicateEvaluator for MatchesEvaluator {
    fn evaluate(&self, context: &mut FileContext, _key: &PredicateKey, value: &str) -> Result<bool> {
        let content = context.get_content()?;
        let re = regex::Regex::new(value)?;
        Ok(re.is_match(content))
    }
}

struct SizeEvaluator;
impl PredicateEvaluator for SizeEvaluator {
    fn evaluate(&self, context: &mut FileContext, _key: &PredicateKey, value: &str) -> Result<bool> {
        let metadata = context.path.metadata()?;
        let file_size = metadata.len();
        parse_and_compare_size(file_size, value)
    }
}

struct ModifiedEvaluator;
impl PredicateEvaluator for ModifiedEvaluator {
    fn evaluate(&self, context: &mut FileContext, _key: &PredicateKey, value: &str) -> Result<bool> {
        let metadata = context.path.metadata()?;
        let modified_time = metadata.modified()?;
        parse_and_compare_time(modified_time, value)
    }
}

fn parse_and_compare_size(file_size: u64, query: &str) -> Result<bool> {
    let (op, size_str) = query.split_at(1);
    let target_size = size_str
        .trim()
        .to_lowercase()
        .replace("kb", " * 1024")
        .replace("mb", " * 1024 * 1024")
        .replace("gb", " * 1024 * 1024 * 1024");

    // A simple expression evaluator for "N * N * N..."
    let target_size_bytes = target_size
        .split('*')
        .map(|s| s.trim().parse::<f64>())
        .collect::<Result<Vec<f64>, _>>()?
        .into_iter()
        .product::<f64>() as u64;

    match op {
        ">" => Ok(file_size > target_size_bytes),
        "<" => Ok(file_size < target_size_bytes),
        "=" => Ok(file_size == target_size_bytes),
        _ => Err(anyhow!("Invalid size operator: {}", op)),
    }
}

fn parse_and_compare_time(modified_time: SystemTime, query: &str) -> Result<bool> {
    let now = SystemTime::now();
    let (op, duration_str) = query.split_at(1);
    let duration_str = duration_str.trim();

    let duration_secs = if let Some(num_str) = duration_str.strip_suffix('s') {
        num_str.parse::<u64>()?
    } else if let Some(num_str) = duration_str.strip_suffix('m') {
        num_str.parse::<u64>()? * 60
    } else if let Some(num_str) = duration_str.strip_suffix('h') {
        num_str.parse::<u64>()? * 3600
    } else if let Some(num_str) = duration_str.strip_suffix('d') {
        num_str.parse::<u64>()? * 86400
    } else {
        return Err(anyhow!("Invalid time unit in '{}'", query));
    };

    let duration = Duration::from_secs(duration_secs);
    let threshold_time = now.checked_sub(duration).ok_or(anyhow!("Time calculation underflow"))?;

    match op {
        ">" => Ok(modified_time > threshold_time), // Modified more recently than
        "<" => Ok(modified_time < threshold_time), // Modified longer ago than
        _ => Err(anyhow!("Invalid time operator: {}", op)),
    }
}

/// Creates and populates the predicate registry.
pub fn create_predicate_registry() -> HashMap<PredicateKey, Box<dyn PredicateEvaluator + Send + Sync>> {
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
    // It's a stateless struct, so creating multiple boxes is cheap.
    registry.insert(PredicateKey::Def, Box::new(CodeAwareEvaluator));
    registry.insert(PredicateKey::Func, Box::new(CodeAwareEvaluator));
    registry.insert(PredicateKey::Import, Box::new(CodeAwareEvaluator));

    registry
}

#[cfg(test)]
mod tests {
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
        assert!(evaluator.evaluate(&mut context, &PredicateKey::Size, ">1000").unwrap());
        assert!(!evaluator.evaluate(&mut context, &PredicateKey::Size, "<1kb").unwrap());
        assert!(evaluator.evaluate(&mut context, &PredicateKey::Size, ">0.9kb").unwrap());
    }

    #[test]
    fn test_modified_evaluator() {
        let file = create_temp_file("content");
        let mut context = FileContext::new(file.path().to_path_buf());

        let evaluator = ModifiedEvaluator;
        // File was just created
        assert!(evaluator.evaluate(&mut context, &PredicateKey::Modified, ">1m").unwrap()); // Modified more recently than 1 min ago
        assert!(!evaluator.evaluate(&mut context, &PredicateKey::Modified, "<1m").unwrap()); // Not modified longer than 1 min ago
    }

    #[test]
    fn test_ext_evaluator() {
        let mut context_rs = FileContext::new(PathBuf::from("main.rs"));
        let mut context_toml = FileContext::new(PathBuf::from("Cargo.TOML"));
        let mut context_no_ext = FileContext::new(PathBuf::from("README"));
        let mut context_dotfile = FileContext::new(PathBuf::from(".bashrc"));

        let evaluator = ExtEvaluator;
        assert!(evaluator.evaluate(&mut context_rs, &PredicateKey::Ext, "rs").unwrap());
        assert!(!evaluator.evaluate(&mut context_rs, &PredicateKey::Ext, "toml").unwrap());
        assert!(evaluator.evaluate(&mut context_toml, &PredicateKey::Ext, "toml").unwrap(), "Should be case-insensitive");
        assert!(!evaluator.evaluate(&mut context_no_ext, &PredicateKey::Ext, "rs").unwrap());
        assert!(!evaluator.evaluate(&mut context_dotfile, &PredicateKey::Ext, "bashrc").unwrap(), "Dotfiles should have no extension");
    }

    #[test]
    fn test_path_evaluator() {
        let mut context = FileContext::new(PathBuf::from("/home/user/project/src/main.rs"));
        let evaluator = PathEvaluator;
        assert!(evaluator.evaluate(&mut context, &PredicateKey::Path, "project/src").unwrap());
        assert!(evaluator.evaluate(&mut context, &PredicateKey::Path, "/home/user").unwrap());
        assert!(!evaluator.evaluate(&mut context, &PredicateKey::Path, "project/lib").unwrap());
        assert!(evaluator.evaluate(&mut context, &PredicateKey::Path, "main.rs").unwrap());
    }

    #[test]
    fn test_name_evaluator() {
        let mut context1 = FileContext::new(PathBuf::from("/home/user/Cargo.toml"));
        let mut context2 = FileContext::new(PathBuf::from("/home/user/main.rs"));

        let evaluator = NameEvaluator;
        assert!(evaluator.evaluate(&mut context1, &PredicateKey::Name, "Cargo.toml").unwrap());
        assert!(evaluator.evaluate(&mut context1, &PredicateKey::Name, "C*.toml").unwrap(), "Glob pattern should match");
        assert!(evaluator.evaluate(&mut context2, &PredicateKey::Name, "*.rs").unwrap(), "Glob pattern should match");
        assert!(!evaluator.evaluate(&mut context1, &PredicateKey::Name, "*.rs").unwrap());
    }

    #[test]
    fn test_contains_evaluator() {
        let file = create_temp_file("Hello world\nThis is a test.");
        let mut context = FileContext::new(file.path().to_path_buf());
        let evaluator = ContainsEvaluator;
        assert!(evaluator.evaluate(&mut context, &PredicateKey::Contains, "world").unwrap());
        assert!(evaluator.evaluate(&mut context, &PredicateKey::Contains, "is a test").unwrap());
        assert!(!evaluator.evaluate(&mut context, &PredicateKey::Contains, "goodbye").unwrap());
    }

    #[test]
    fn test_matches_evaluator() {
        let file = create_temp_file("version = \"0.1.0\"\nauthor = \"test\"");
        let mut context = FileContext::new(file.path().to_path_buf());
        let evaluator = MatchesEvaluator;
        // Simple regex
        assert!(evaluator.evaluate(&mut context, &PredicateKey::Matches, "version = \"[0-9]+\\.[0-9]+\\.[0-9]+\"").unwrap());
        // Test regex that spans lines
        assert!(evaluator.evaluate(&mut context, &PredicateKey::Matches, "(?s)version.*author").unwrap());
        assert!(!evaluator.evaluate(&mut context, &PredicateKey::Matches, "^version = \"1.0.0\"$").unwrap());
    }

    #[test]
    fn test_code_aware_evaluator_rust_def() {
        let rust_code = "struct User; enum Role {}";

        // Create a temp file with a .rs extension
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("code.rs");
        let mut file = std::fs::File::create(&file_path).unwrap();
        file.write_all(rust_code.as_bytes()).unwrap();

        let mut context = FileContext::new(file_path.clone());
        let evaluator = CodeAwareEvaluator;

        // Test successful matches
        assert!(evaluator.evaluate(&mut context, &PredicateKey::Def, "User").unwrap(), "Should find struct User");

        // Reset context for the next evaluation on the same file
        let mut context = FileContext::new(file_path);
        assert!(evaluator.evaluate(&mut context, &PredicateKey::Def, "Role").unwrap(), "Should find enum Role");
    }

    #[test]
    fn test_code_aware_evaluator_full_rust_suite() {
        let rust_code = "\n            use std::collections::HashMap;\n            use serde::{Serialize, Deserialize};\n\n            struct AppConfig {}\n            trait Runnable {\n                fn run(&self);\n            }\n            fn launch_app() {}\n        ";

        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("complex.rs");
        let mut file = std::fs::File::create(&file_path).unwrap();
        file.write_all(rust_code.as_bytes()).unwrap();

        let evaluator = CodeAwareEvaluator;

        // --- Test Definitions ---
        let mut ctx = FileContext::new(file_path.clone());
        assert!(evaluator.evaluate(&mut ctx, &PredicateKey::Def, "AppConfig").unwrap());
        let mut ctx = FileContext::new(file_path.clone());
        assert!(evaluator.evaluate(&mut ctx, &PredicateKey::Def, "Runnable").unwrap());

        // --- Test Functions ---
        let mut ctx = FileContext::new(file_path.clone());
        assert!(evaluator.evaluate(&mut ctx, &PredicateKey::Func, "run").unwrap(), "Should find trait method");
        let mut ctx = FileContext::new(file_path.clone());
        assert!(evaluator.evaluate(&mut ctx, &PredicateKey::Func, "launch_app").unwrap(), "Should find standalone function");
        let mut ctx = FileContext::new(file_path.clone());
        assert!(!evaluator.evaluate(&mut ctx, &PredicateKey::Func, "AppConfig").unwrap());

        // --- Test Imports ---
        let mut ctx = FileContext::new(file_path.clone());
        assert!(evaluator.evaluate(&mut ctx, &PredicateKey::Import, "std::collections").unwrap());
        let mut ctx = FileContext::new(file_path.clone());
        assert!(evaluator.evaluate(&mut ctx, &PredicateKey::Import, "serde").unwrap(), "Should match part of a use statement");
        let mut ctx = FileContext::new(file_path.clone());
        assert!(evaluator.evaluate(&mut ctx, &PredicateKey::Import, "Serialize").unwrap(), "Should match item in a use list");
        let mut ctx = FileContext::new(file_path.clone());
         assert!(!evaluator.evaluate(&mut ctx, &PredicateKey::Import, "anyhow").unwrap());
     }

    #[test]
    fn test_code_aware_evaluator_python_suite() {
        let python_code = r#"
import os
from sys import argv

class DataProcessor:
    def __init__(self):
        pass

def process_data():
    print("Processing")
        "#;

        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("script.py");
        let mut file = std::fs::File::create(&file_path).unwrap();
        file.write_all(python_code.as_bytes()).unwrap();

        let evaluator = CodeAwareEvaluator;

        // --- Test Definitions (Classes) ---
        let mut ctx = FileContext::new(file_path.clone());
        assert!(evaluator.evaluate(&mut ctx, &PredicateKey::Def, "DataProcessor").unwrap());
        let mut ctx = FileContext::new(file_path.clone());
        assert!(!evaluator.evaluate(&mut ctx, &PredicateKey::Def, "process_data").unwrap());

        // --- Test Functions ---
        let mut ctx = FileContext::new(file_path.clone());
        assert!(evaluator.evaluate(&mut ctx, &PredicateKey::Func, "process_data").unwrap());
        let mut ctx = FileContext::new(file_path.clone());
        assert!(!evaluator.evaluate(&mut ctx, &PredicateKey::Func, "DataProcessor").unwrap());

        // --- Test Imports ---
        let mut ctx = FileContext::new(file_path.clone());
        assert!(evaluator.evaluate(&mut ctx, &PredicateKey::Import, "os").unwrap());
        let mut ctx = FileContext::new(file_path.clone());
        assert!(evaluator.evaluate(&mut ctx, &PredicateKey::Import, "sys").unwrap());
        let mut ctx = FileContext::new(file_path.clone());
        assert!(evaluator.evaluate(&mut ctx, &PredicateKey::Import, "argv").unwrap());
        let mut ctx = FileContext::new(file_path.clone());
        assert!(!evaluator.evaluate(&mut ctx, &PredicateKey::Import, "numpy").unwrap());
    }
 }