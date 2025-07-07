use anyhow::{anyhow, Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tree_sitter::{Parser as TreeSitterParser, Range, Tree};

use crate::parser::{AstNode, PredicateKey};
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

    /// Evaluates the query for a given file path, but only for metadata predicates.
    pub fn pre_filter_evaluate(&self, context: &mut FileContext) -> Result<bool> {
        self.pre_filter_evaluate_node(&self.ast, context)
    }

    /// Recursively evaluates an AST node for the pre-filtering pass.
    fn pre_filter_evaluate_node(&self, node: &AstNode, context: &mut FileContext) -> Result<bool> {
        match node {
            AstNode::Predicate(key, value) => {
                if let Some(evaluator) = self.registry.get(key) {
                    Ok(evaluator.evaluate(context, key, value)?.is_match())
                } else {
                    // If a predicate is not in the metadata registry, we can't evaluate it.
                    // We must assume it *could* match and let the full evaluator decide.
                    Ok(true)
                }
            }
            AstNode::LogicalOp(op, left, right) => {
                match op {
                    crate::parser::LogicalOperator::And => {
                        Ok(self.pre_filter_evaluate_node(left, context)? && self.pre_filter_evaluate_node(right, context)?)
                    }
                    crate::parser::LogicalOperator::Or => {
                        Ok(self.pre_filter_evaluate_node(left, context)? || self.pre_filter_evaluate_node(right, context)?)
                    }
                }
            }
            AstNode::Not(inner_node) => {
                // For the pre-filtering pass, if the inner predicate of a NOT is not in the
                // registry, we cannot definitively say the file *doesn't* match.
                // For example, for `!contains:foo`, the pre-filter doesn't know the content.
                // So, we must assume it *could* match and let the full evaluator decide.
                if let AstNode::Predicate(key, _) = &**inner_node {
                    if !self.registry.contains_key(key) {
                        return Ok(true); // Pass to the next stage
                    }
                }
                Ok(!self.pre_filter_evaluate_node(inner_node, context)?)
            }
        }
    }

    /// Recursively evaluates an AST node.
    fn evaluate_node(&self, node: &AstNode, context: &mut FileContext) -> Result<MatchResult> {
        match node {
            AstNode::Predicate(key, value) => self.evaluate_predicate(key, value, context),
            AstNode::LogicalOp(op, left, right) => {
                match op {
                    crate::parser::LogicalOperator::And => {
                        let left_res = self.evaluate_node(left, context)?;
                        if !left_res.is_match() {
                            return Ok(MatchResult::Boolean(false));
                        }
                        let right_res = self.evaluate_node(right, context)?;
                        if !right_res.is_match() {
                            return Ok(MatchResult::Boolean(false));
                        }
                        Ok(left_res.combine_with(right_res, op))
                    }
                    crate::parser::LogicalOperator::Or => {
                        let left_res = self.evaluate_node(left, context)?;
                        // Short-circuit if we have a non-hunkable, definitive match.
                        // This prevents expensive evaluation of the right side.
                        if let MatchResult::Boolean(true) = left_res {
                            return Ok(left_res);
                        }

                        let right_res = self.evaluate_node(right, context)?;

                        // Combine the results logically.
                        if left_res.is_match() && right_res.is_match() {
                            Ok(left_res.combine_with(right_res, op))
                        } else if left_res.is_match() {
                            Ok(left_res) // right side didn't match
                        } else {
                            Ok(right_res) // left side didn't match, so result is right
                        }
                    }
                }
            }
            AstNode::Not(inner_node) => {
                // Evaluate the inner node and negate the result.
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
            // The full evaluator in the next stage will make the final decision.
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

    /// Combines two successful match results.
    pub fn combine_with(self, other: MatchResult, op: &crate::parser::LogicalOperator) -> Self {
        match (self, other) {
            (MatchResult::Hunks(mut a), MatchResult::Hunks(b)) => {
                match op {
                    crate::parser::LogicalOperator::And => {
                        a.retain(|hunk_a| b.iter().any(|hunk_b| Self::hunks_overlap(hunk_a, hunk_b)));
                    }
                    crate::parser::LogicalOperator::Or => {
                        a.extend(b);
                        a.sort_by_key(|r| r.start_byte);
                        a.dedup();
                    }
                }
                MatchResult::Hunks(a)
            }
            (MatchResult::Hunks(a), MatchResult::Boolean(_)) => MatchResult::Hunks(a),
            (MatchResult::Boolean(_), MatchResult::Hunks(b)) => MatchResult::Hunks(b),
            (MatchResult::Boolean(a), MatchResult::Boolean(b)) => {
                match op {
                    crate::parser::LogicalOperator::And => MatchResult::Boolean(a && b),
                    crate::parser::LogicalOperator::Or => MatchResult::Boolean(a || b),
                }
            }
        }
    }

    fn hunks_overlap(a: &Range, b: &Range) -> bool {
        a.start_byte < b.end_byte && b.start_byte < a.end_byte
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
    fn test_evaluate_simple_predicate() {
        let file = create_temp_file("hello world");
        let mut context = FileContext::new(file.path().to_path_buf());
        let ast = parse_query("contains:world").unwrap();
        let evaluator = Evaluator::new(ast, predicates::create_predicate_registry());
        assert!(evaluator.evaluate(&mut context).unwrap().is_match());
    }

    #[test]
    fn test_evaluate_logical_and() {
        let file = create_temp_file("hello world");
        let mut context = FileContext::new(file.path().to_path_buf());
        let ast = parse_query("contains:hello & contains:world").unwrap();
        let evaluator = Evaluator::new(ast, predicates::create_predicate_registry());
        assert!(evaluator.evaluate(&mut context).unwrap().is_match());

        let ast_fail = parse_query("contains:hello & contains:goodbye").unwrap();
        let evaluator_fail = Evaluator::new(ast_fail, predicates::create_predicate_registry());
        assert!(!evaluator_fail
            .evaluate(&mut context)
            .unwrap()
            .is_match());
    }

    #[test]
    fn test_evaluate_logical_or() {
        let file = create_temp_file("hello world");
        let mut context = FileContext::new(file.path().to_path_buf());
        let ast = parse_query("contains:hello | contains:goodbye").unwrap();
        let evaluator = Evaluator::new(ast, predicates::create_predicate_registry());
        assert!(evaluator.evaluate(&mut context).unwrap().is_match());

        let ast_fail = parse_query("contains:goodbye | contains:farewell").unwrap();
        let evaluator_fail = Evaluator::new(ast_fail, predicates::create_predicate_registry());
        assert!(!evaluator_fail
            .evaluate(&mut context)
            .unwrap()
            .is_match());
    }

    #[test]
    fn test_evaluate_negation() {
        let file = create_temp_file("hello world");
        let mut context = FileContext::new(file.path().to_path_buf());
        let ast = parse_query("!contains:goodbye").unwrap();
        let evaluator = Evaluator::new(ast, predicates::create_predicate_registry());
        assert!(evaluator.evaluate(&mut context).unwrap().is_match());

        let ast_fail = parse_query("!contains:hello").unwrap();
        let evaluator_fail = Evaluator::new(ast_fail, predicates::create_predicate_registry());
        assert!(!evaluator_fail
            .evaluate(&mut context)
            .unwrap()
            .is_match());
    }

    #[test]
    fn test_combine_with_hunks_intersection() {
        let hunks1 = vec![tree_sitter::Range { start_byte: 10, end_byte: 20, start_point: Default::default(), end_point: Default::default() }];
        let hunks2 = vec![tree_sitter::Range { start_byte: 15, end_byte: 25, start_point: Default::default(), end_point: Default::default() }];
        let result1 = MatchResult::Hunks(hunks1);
        let result2 = MatchResult::Hunks(hunks2);
        let combined = result1.combine_with(result2, &crate::parser::LogicalOperator::And);
        assert!(combined.is_match());
        if let MatchResult::Hunks(hunks) = combined {
            assert_eq!(hunks.len(), 1);
        } else {
            panic!("Expected Hunks result");
        }
    }
}