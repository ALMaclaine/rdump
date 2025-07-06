use anyhow::{Context, Result};
use regex;
use std::fs;
use std::path::{Path, PathBuf};

use crate::parser::{AstNode, PredicateKey};

/// A context for a single file being evaluated.
/// It lazily loads metadata and content to avoid unnecessary I/O.
#[derive(Debug)]
pub struct FileContext {
    pub path: PathBuf,
    content: Option<String>,
}

// THE CORRECTED LINE:
impl FileContext {
    /// Creates a new context for a given path.
    pub fn new(path: PathBuf) -> Self {
        FileContext {
            path,
            content: None,
        }
    }

    /// Lazily gets the file content, reading it from disk only once.
    fn get_content(&mut self) -> Result<&str> {
        if self.content.is_none() {
            let content = fs::read_to_string(&self.path)
                .with_context(|| format!("Failed to read file content: {}", self.path.display()))?;
            self.content = Some(content);
        }
        Ok(self.content.as_ref().unwrap())
    }
}

/// The main evaluator struct. It holds the parsed query AST.
pub struct Evaluator<'a> {
    ast: &'a AstNode,
}

impl<'a> Evaluator<'a> {
    /// Creates a new evaluator with a reference to the AST.
    pub fn new(ast: &'a AstNode) -> Self {
        Self { ast }
    }

    /// Evaluates a single file path against the AST.
    pub fn evaluate(&self, path: &Path) -> Result<bool> {
        let mut context = FileContext::new(path.to_path_buf());
        self.evaluate_node(self.ast, &mut context)
    }

    /// The core recursive function that walks the AST.
    fn evaluate_node(&self, node: &AstNode, context: &mut FileContext) -> Result<bool> {
        match node {
            AstNode::And(left, right) => {
                Ok(self.evaluate_node(left, context)? && self.evaluate_node(right, context)?)
            }
            AstNode::Or(left, right) => {
                Ok(self.evaluate_node(left, context)? || self.evaluate_node(right, context)?)
            }
            AstNode::Not(node) => Ok(!self.evaluate_node(node, context)?),
            AstNode::Predicate { key, value } => self.evaluate_predicate(key, value, context),
        }
    }

    /// Dispatches to the correct logic for each predicate type.
    fn evaluate_predicate(
        &self,
        key: &PredicateKey,
        value: &str,
        context: &mut FileContext,
    ) -> Result<bool> {
        match key {
            PredicateKey::Ext => {
                let file_ext = context
                    .path
                    .extension()
                    .and_then(|s| s.to_str())
                    .unwrap_or("");
                Ok(file_ext.eq_ignore_ascii_case(value))
            }
            PredicateKey::Path => {
                let path_str = context.path.to_string_lossy();
                Ok(path_str.contains(value))
            }
            PredicateKey::Contains => {
                let content = context.get_content()?;
                Ok(content.contains(value))
            }
            // --- NEW IMPLEMENTATIONS ---
            PredicateKey::Name => {
                let file_name = context
                    .path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("");
                // Use the `glob` crate to match the pattern.
                let pattern = glob::Pattern::new(value)
                    .with_context(|| format!("Invalid glob pattern: '{}'", value))?;
                Ok(pattern.matches(file_name))
            }
            PredicateKey::Matches => {
                // Use the `regex` crate to match the pattern.
                let content = context.get_content()?;
                let re = regex::Regex::new(value)
                    .with_context(|| format!("Invalid regex pattern: '{}'", value))?;
                Ok(re.is_match(content))
            }
            // --- END NEW ---
            PredicateKey::Other(unknown_key) => {
                // Handle unknown predicates gracefully.
                println!("Warning: unknown predicate key '{}'", unknown_key);
                Ok(false)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;
    use std::io::Write;
    use tempfile::{Builder, NamedTempFile};

    // Helper to create a temporary file with specific content for a test.
    fn create_temp_file(content: &str, extension: &str) -> NamedTempFile {
        let mut file = Builder::new()
            .prefix("rdump_test_")
            .suffix(&format!(".{}", extension))
            .tempfile()
            .unwrap();
        file.write_all(content.as_bytes()).unwrap();
        file
    }

    // Helper to run a query against a file and assert the result.
    fn assert_eval(query: &str, file_path: &Path, expected: bool) {
        let ast = parser::parse_query(query).unwrap();
        let evaluator = Evaluator::new(&ast);
        let result = evaluator.evaluate(file_path).unwrap();
        assert_eq!(
            result,
            expected,
            "Query: '{}' on file '{}'",
            query,
            file_path.display()
        );
    }

    #[test]
    fn test_predicate_ext() {
        let file = create_temp_file("hello world", "rs");
        assert_eval("ext:rs", file.path(), true);
        assert_eval("ext:toml", file.path(), false);
        assert_eval("ext:RS", file.path(), true);
    }

    #[test]
    fn test_predicate_path() {
        let file = create_temp_file(r#"some content"#, "txt");
        let path_str = file.path().to_string_lossy();

        let path_segment = path_str.split(std::path::MAIN_SEPARATOR).last().unwrap();

        assert_eval(&format!("path:{}", path_segment), file.path(), true);
        assert_eval("path:this_will_not_exist", file.path(), false);
    }

    #[test]
    fn test_predicate_contains() {
        let file = create_temp_file("hello rust world", "txt");
        assert_eval("contains:rust", file.path(), true);
        assert_eval("contains:'hello world'", file.path(), false);
        assert_eval("contains:goodbye", file.path(), false);
    }

    #[test]
    fn test_logical_and() {
        let file = create_temp_file("fn main() {}", "rs");
        assert_eval("ext:rs & contains:main", file.path(), true);
        assert_eval("ext:rs & contains:goodbye", file.path(), false);
        assert_eval("ext:toml & contains:main", file.path(), false);
    }

    #[test]
    fn test_logical_or() {
        let file = create_temp_file("some toml content", "toml");
        assert_eval("ext:rs | contains:toml", file.path(), true);
        assert_eval("ext:toml | contains:rust", file.path(), true);
        assert_eval("ext:rs | contains:rust", file.path(), false);
    }

    #[test]
    fn test_logical_not() {
        let file = create_temp_file("hello", "md");
        assert_eval("!ext:rs", file.path(), true);
        assert_eval("!ext:md", file.path(), false);
        assert_eval("!(ext:rs | ext:toml)", file.path(), true);
        assert_eval("!(ext:md | ext:toml)", file.path(), false);
    }

    #[test]
    fn test_complex_query() {
        let file = create_temp_file("public fn start()", "rs");
        let query = "ext:rs & !path:tests & contains:'fn'";
        assert_eval(query, file.path(), true);

        let query = "(ext:rs & contains:struct) | ext:toml";
        assert_eval(query, file.path(), false);
    }

    #[test]
    fn test_lazy_content_loading() {
        let file = create_temp_file("expensive content", "txt");
        let ast = parser::parse_query("ext:rs & contains:expensive").unwrap();
        let evaluator = Evaluator::new(&ast);
        let result = evaluator.evaluate(file.path()).unwrap();
        assert_eq!(
            result, false,
            "Should short-circuit and not evaluate contains"
        );
    }

    #[test]
    fn test_predicate_name_glob() {
        let file = create_temp_file("content", "rs");
        let file_name = file.path().file_name().unwrap().to_str().unwrap();

        // Exact match
        assert_eval(&format!("name:'{}'", file_name), file.path(), true);
        // Glob match
        assert_eval("name:'*_test_*.rs'", file.path(), true);
        assert_eval("name:'*.rs'", file.path(), true);
        assert_eval("name:'*.toml'", file.path(), false);
    }

    #[test]
    fn test_predicate_matches_regex() {
        let file = create_temp_file("hello 123 world", "txt");
        // Matches a digit
        assert_eval(r#"matches:'\d+'"#, file.path(), true);
        // Matches start of string
        assert_eval("matches:'^hello'", file.path(), true);
        // Does not match
        assert_eval("matches:'^world'", file.path(), false);
        // Invalid regex should not panic, but return an error (which assert_eval would unwrap)
        // A more robust test could check for the specific error.
        let ast = parser::parse_query("matches:'('").unwrap();
        let evaluator = Evaluator::new(&ast);
        let result = evaluator.evaluate(file.path());
        assert!(result.is_err(), "Invalid regex should produce an error");
    }

    #[test]
    fn test_unknown_predicate_is_false() {
        let file = create_temp_file("some content", "txt");
        // Our parser turns `foo:bar` into `PredicateKey::Other("foo")`
        // The evaluator should see this and return false.
        assert_eval("foo:bar", file.path(), false);
    }
}
