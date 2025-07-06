use anyhow::{Result, Context};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::parser::{AstNode, PredicateKey};
use crate::predicates::{create_predicate_registry, PredicateEvaluator};

/// A context for a single file being evaluated.
#[derive(Debug)]
pub struct FileContext {
    pub path: PathBuf,
    content: Option<String>,
}

impl FileContext {
    pub fn new(path: PathBuf) -> Self {
        FileContext { path, content: None }
    }

    pub fn get_content(&mut self) -> Result<&str> {
        if self.content.is_none() {
            let content = fs::read_to_string(&self.path)
                .with_context(|| format!("Failed to read content of {}", self.path.display()))?;
            self.content = Some(content);
        }
        Ok(self.content.as_ref().unwrap())
    }
}

/// The main evaluator struct. It holds the AST and the predicate registry.
pub struct Evaluator<'a> {
    ast: &'a AstNode,
    // The registry of all available predicate "plugins".
    registry: HashMap<PredicateKey, Box<dyn PredicateEvaluator + Send + Sync>>,
}

impl<'a> Evaluator<'a> {
    /// Creates a new evaluator with a reference to the AST.
    pub fn new(ast: &'a AstNode) -> Self {
        Self {
            ast,
            registry: create_predicate_registry(),
        }
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

    /// Dispatches to the correct plugin from the registry.
    fn evaluate_predicate(
        &self,
        key: &PredicateKey,
        value: &str,
        context: &mut FileContext,
    ) -> Result<bool> {
        if let Some(evaluator) = self.registry.get(key) {
            evaluator.evaluate(context, value)
        } else {
            // Handle unknown or unimplemented predicates gracefully.
            if let PredicateKey::Other(unknown_key) = key {
                 println!("Warning: unknown predicate key '{}'", unknown_key);
            }
            Ok(false)
        }
    }
}

// --- TESTS ARE NOW MOVED ---
// All the old evaluator tests are now invalid because the logic has moved.
// We will create new, more focused tests inside the `predicates` module itself.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_query;
    use std::io::Write;
    use tempfile::NamedTempFile;

    // Helper to create a file with specific content for testing
    fn create_test_file(content: &str) -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();
        file
    }

    #[test]
    fn test_logical_operators() {
        let file = create_test_file("fn main() {}");
        // Give the temp file a `.rs` extension for the ext: predicate
        let path_rs = file.path().with_extension("rs");
        std::fs::rename(file.path(), &path_rs).unwrap();

        // true & true => true
        let ast_and_true = parse_query("ext:rs & contains:'main'").unwrap();
        let evaluator_and_true = Evaluator::new(&ast_and_true);
        assert!(evaluator_and_true.evaluate(&path_rs).unwrap());

        // true & false => false
        let ast_and_false = parse_query("ext:rs & contains:'other'").unwrap();
        let evaluator_and_false = Evaluator::new(&ast_and_false);
        assert!(!evaluator_and_false.evaluate(&path_rs).unwrap());

        // true | false => true
        let ast_or_true = parse_query("ext:rs | contains:'other'").unwrap();
        let evaluator_or_true = Evaluator::new(&ast_or_true);
        assert!(evaluator_or_true.evaluate(&path_rs).unwrap());

        // false | false => false
        let ast_or_false = parse_query("ext:md | contains:'other'").unwrap();
        let evaluator_or_false = Evaluator::new(&ast_or_false);
        assert!(!evaluator_or_false.evaluate(&path_rs).unwrap());

        // !false => true
        let ast_not_true = parse_query("!ext:md").unwrap();
        let evaluator_not_true = Evaluator::new(&ast_not_true);
        assert!(evaluator_not_true.evaluate(&path_rs).unwrap());

        // !true => false
        let ast_not_false = parse_query("!ext:rs").unwrap();
        let evaluator_not_false = Evaluator::new(&ast_not_false);
        assert!(!evaluator_not_false.evaluate(&path_rs).unwrap());
    }

    #[test]
    fn test_file_context_lazy_loading() {
        let file = create_test_file("lazy content");
        let mut context = FileContext::new(file.path().to_path_buf());

        // Content is None initially
        assert!(context.content.is_none());

        // get_content loads it
        let content = context.get_content().unwrap();
        assert_eq!(content, "lazy content");
        assert!(context.content.is_some());

        // Calling it again returns the same content without re-reading (implicitly)
        let content_again = context.get_content().unwrap();
        assert_eq!(content_again, "lazy content");
    }
}
