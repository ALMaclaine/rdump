use anyhow::{anyhow, Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tree_sitter::{Parser, Range, Tree};

use crate::parser::{AstNode, LogicalOperator, PredicateKey};
use crate::predicates::PredicateEvaluator;

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
    pub root: PathBuf,
    content: Option<String>,
    // Cache for the parsed tree-sitter AST
    tree: Option<Tree>,
}

impl FileContext {
    pub fn new(path: PathBuf, root: PathBuf) -> Self {
        FileContext {
            path,
            root,
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
            let mut parser = Parser::new();
            parser.set_language(&language).with_context(|| {
                format!("Failed to set language for tree-sitter parser on {path_display}")
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
    pub fn new(
        ast: AstNode,
        registry: HashMap<PredicateKey, Box<dyn PredicateEvaluator + Send + Sync>>,
    ) -> Self {
        Evaluator { ast, registry }
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

                // Short-circuit AND if left is false
                if *op == LogicalOperator::And && !left_res.is_match() {
                    return Ok(MatchResult::Boolean(false));
                }

                // Short-circuit OR if left is a full-file match
                if *op == LogicalOperator::Or {
                    if let MatchResult::Boolean(true) = left_res {
                        return Ok(left_res);
                    }
                }

                let right_res = self.evaluate_node(right, context)?;
                Ok(left_res.combine_with(right_res, op))
            }
            AstNode::Not(inner_node) => {
                // If the inner predicate of a NOT is not in the registry (e.g., a content
                // predicate during the metadata-only pass), we cannot definitively say the file
                // *doesn't* match. We must assume it *could* match and let the full evaluator decide.
                if let AstNode::Predicate(key, _) = &**inner_node {
                    if !self.registry.contains_key(key) {
                        return Ok(MatchResult::Boolean(true));
                    }
                }
                let result = self.evaluate_node(inner_node, context)?;
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
            // If a predicate is not in the current registry (e.g., a content predicate
            // during the metadata-only pass), it's considered a "pass" for this stage.
            Ok(MatchResult::Boolean(true))
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

    /// Combines two match results based on a logical operator.
    pub fn combine_with(self, other: MatchResult, op: &LogicalOperator) -> Self {
        match op {
            LogicalOperator::And => {
                if !self.is_match() || !other.is_match() {
                    return MatchResult::Boolean(false);
                }
                match (self, other) {
                    (MatchResult::Hunks(mut a), MatchResult::Hunks(b)) => {
                        a.extend(b);
                        a.sort_by_key(|r| r.start_byte);
                        a.dedup();
                        MatchResult::Hunks(a)
                    }
                    (h @ MatchResult::Hunks(_), MatchResult::Boolean(true)) => h,
                    (MatchResult::Boolean(true), h @ MatchResult::Hunks(_)) => h,
                    (MatchResult::Boolean(true), MatchResult::Boolean(true)) => MatchResult::Boolean(true),
                    _ => MatchResult::Boolean(false),
                }
            }
            LogicalOperator::Or => {
                match (self, other) {
                    (MatchResult::Boolean(true), _) | (_, MatchResult::Boolean(true)) => MatchResult::Boolean(true),
                    (MatchResult::Hunks(mut a), MatchResult::Hunks(b)) => {
                        a.extend(b);
                        a.sort_by_key(|r| r.start_byte);
                        a.dedup();
                        MatchResult::Hunks(a)
                    }
                    (h @ MatchResult::Hunks(_), MatchResult::Boolean(false)) => h,
                    (MatchResult::Boolean(false), h @ MatchResult::Hunks(_)) => h,
                    (MatchResult::Boolean(false), MatchResult::Boolean(false)) => MatchResult::Boolean(false),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_query;
    use crate::predicates;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_temp_file(content: &str) -> NamedTempFile {
        let file = NamedTempFile::new().unwrap();
        write!(file.as_file(), "{}", content).unwrap();
        file
    }

    #[test]
    fn test_evaluate_logical_and() {
        let file = create_temp_file("hello world");
        let mut context = FileContext::new(file.path().to_path_buf(), PathBuf::from("/"));
        let ast = parse_query("contains:hello & contains:world").unwrap();
        let evaluator = Evaluator::new(ast, predicates::create_predicate_registry());
        assert!(evaluator.evaluate(&mut context).unwrap().is_match());

        let ast_fail = parse_query("contains:hello & contains:goodbye").unwrap();
        let evaluator_fail = Evaluator::new(ast_fail, predicates::create_predicate_registry());
        assert!(!evaluator_fail.evaluate(&mut context).unwrap().is_match());
    }
}
