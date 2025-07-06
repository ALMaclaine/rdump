use anyhow::{anyhow, Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tree_sitter::{Parser as TreeSitterParser, Range, Tree};

use crate::parser::{AstNode, PredicateKey};
use crate::predicates::{create_predicate_registry, PredicateEvaluator};

/// The result of an evaluation for a single file.
#[derive(Debug, Clone)]
pub enum MatchResult {
    // For simple, non-hunkable predicates like `ext:rs` or `size:>10kb`
    Boolean(bool),
    // For code-aware predicates that can identify specific code blocks.
    Hunks(Vec<Range>),
}

/// Holds the context for a single file being evaluated.
/// It lazily loads content and caches the tree-sitter AST.
pub struct FileContext {
    pub path: PathBuf,
    content: Option<String>,
    // Cache for the parsed tree-sitter AST
    tree: Option<Tree>,
}

impl FileContext {
    pub fn new(path: PathBuf) -> Self {
        FileContext {
            path,
            content: None,
            tree: None,
        }
    }

    pub fn get_content(&mut self) -> Result<&str> {
        if self.content.is_none() {
            let content = fs::read_to_string(&self.path)
                .with_context(|| format!("Failed to read file {}", self.path.display()))?;
            self.content = Some(content);
        }
        Ok(self.content.as_ref().unwrap())
    }

    // Lazily parses the file with tree-sitter and caches the result.
    pub fn get_tree(&mut self, language: tree_sitter::Language) -> Result<&Tree> {
        if self.tree.is_none() {
            let path_display = self.path.display().to_string();
            let content = self.get_content()?;
            let mut parser = TreeSitterParser::new();
            parser.set_language(&language).with_context(|| {
                format!(
                    "Failed to set language for tree-sitter parser on {}",
                    path_display
                )
            })?;
            let tree = parser
                .parse(content, None)
                .ok_or_else(|| anyhow!("Tree-sitter failed to parse {}", path_display))?;
            self.tree = Some(tree);
        }
        Ok(self.tree.as_ref().unwrap())
    }
}

/// The main evaluator struct. It holds the AST and the predicate registry.
pub struct Evaluator {
    ast: AstNode,
    registry: HashMap<PredicateKey, Box<dyn PredicateEvaluator + Send + Sync>>,
}

impl Evaluator {
    pub fn new(ast: AstNode) -> Self {
        Evaluator {
            ast,
            registry: create_predicate_registry(),
        }
    }

    /// Evaluates the query for a given file path.
    pub fn evaluate(&self, context: &mut FileContext) -> Result<MatchResult> {
        self.evaluate_node(&self.ast, context)
    }

    /// Recursively evaluates an AST node.
    fn evaluate_node(&self, node: &AstNode, context: &mut FileContext) -> Result<MatchResult> {
        match node {
            AstNode::Predicate(key, value) => self.evaluate_predicate(key, value, context),
           AstNode::LogicalOp(op, left, right) => {
               let left_res = self.evaluate_node(left, context)?;
               match op {
                   crate::parser::LogicalOperator::And => {
                       // Short-circuit if the left side is false or has no hunks
                       if !left_res.is_match() {
                           return Ok(MatchResult::Boolean(false));
                       }
                       let right_res = self.evaluate_node(right, context)?;
                       if !right_res.is_match() {
                           return Ok(MatchResult::Boolean(false));
                       }
                       // Both sides match, combine the results.
                       return Ok(left_res.combine_with(right_res));
                   }
                   crate::parser::LogicalOperator::Or => {
                       if left_res.is_match() {
                           // If the left side matches, we might still need the right side's hunks.
                           let right_res = self.evaluate_node(right, context)?;
                           return Ok(left_res.combine_with(right_res));
                       }
                       // Left side didn't match, so the result is just the right side.
                       return self.evaluate_node(right, context);
                   }
               }
           }
           AstNode::Not(node) => {
               let result = self.evaluate_node(node, context)?;
               Ok(MatchResult::Boolean(!result.is_match()))
           }
        }
    }

    /// Evaluates a single predicate.
    fn evaluate_predicate(
        &self,
        key: &PredicateKey,
        value: &str,
        context: &mut FileContext,
   ) -> Result<MatchResult> {
        if let Some(evaluator) = self.registry.get(key) {
            evaluator.evaluate(context, key, value)
        } else {
            // Handle unknown or unimplemented predicates gracefully.
           Ok(MatchResult::Boolean(false))
        }
    }
}

impl MatchResult {
   /// Returns true if the result is considered a match.
   pub fn is_match(&self) -> bool {
       match self {
           MatchResult::Boolean(b) => *b,
           MatchResult::Hunks(h) => !h.is_empty(),
       }
   }

   /// Combines two successful match results.
   pub fn combine_with(self, other: MatchResult) -> Self {
       match (self, other) {
           (MatchResult::Hunks(mut a), MatchResult::Hunks(b)) => {
               a.extend(b);
               a.sort_by_key(|r| r.start_byte);
               a.dedup();
               MatchResult::Hunks(a)
           }
           (MatchResult::Hunks(a), MatchResult::Boolean(_)) => MatchResult::Hunks(a),
           (MatchResult::Boolean(_), MatchResult::Hunks(b)) => MatchResult::Hunks(b),
           (MatchResult::Boolean(_), MatchResult::Boolean(_)) => MatchResult::Boolean(true),
       }
   }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_query;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_temp_file(content: &str) -> NamedTempFile {
        let file = NamedTempFile::new().unwrap();
        write!(file.as_file(), "{}", content).unwrap();
        file
    }

    #[test]
    fn test_evaluate_simple_predicate() {
        let file = create_temp_file("hello world");
        let mut context = FileContext::new(file.path().to_path_buf());
        let ast = parse_query("contains:world").unwrap();
        let evaluator = Evaluator::new(ast);
        assert!(evaluator.evaluate(&mut context).unwrap().is_match());
    }

    #[test]
    fn test_evaluate_logical_and() {
        let file = create_temp_file("hello world");
        let mut context = FileContext::new(file.path().to_path_buf());
        let ast = parse_query("contains:hello & contains:world").unwrap();
        let evaluator = Evaluator::new(ast);
        assert!(evaluator.evaluate(&mut context).unwrap().is_match());

        let ast_fail = parse_query("contains:hello & contains:goodbye").unwrap();
        let evaluator_fail = Evaluator::new(ast_fail);
        assert!(!evaluator_fail.evaluate(&mut context).unwrap().is_match());
    }

    #[test]
    fn test_evaluate_logical_or() {
        let file = create_temp_file("hello world");
        let mut context = FileContext::new(file.path().to_path_buf());
        let ast = parse_query("contains:hello | contains:goodbye").unwrap();
        let evaluator = Evaluator::new(ast);
        assert!(evaluator.evaluate(&mut context).unwrap().is_match());

        let ast_fail = parse_query("contains:goodbye | contains:farewell").unwrap();
        let evaluator_fail = Evaluator::new(ast_fail);
        assert!(!evaluator_fail.evaluate(&mut context).unwrap().is_match());
    }

    #[test]
    fn test_evaluate_negation() {
        let file = create_temp_file("hello world");
        let mut context = FileContext::new(file.path().to_path_buf());
        let ast = parse_query("!contains:goodbye").unwrap();
        let evaluator = Evaluator::new(ast);
        assert!(evaluator.evaluate(&mut context).unwrap().is_match());

        let ast_fail = parse_query("!contains:hello").unwrap();
        let evaluator_fail = Evaluator::new(ast_fail);
        assert!(!evaluator_fail.evaluate(&mut context).unwrap().is_match());
    }
}
