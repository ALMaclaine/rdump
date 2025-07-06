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