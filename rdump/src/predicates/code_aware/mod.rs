use crate::evaluator::{FileContext, MatchResult};
use crate::parser::PredicateKey;
use crate::predicates::PredicateEvaluator;
use anyhow::{Context, Result};
use tree_sitter::{Query, QueryCursor};

pub mod profiles;

/// The evaluator that uses tree-sitter to perform code-aware queries.
#[derive(Debug, Clone)]
pub struct CodeAwareEvaluator;

impl PredicateEvaluator for CodeAwareEvaluator {
    fn evaluate(
        &self,
        context: &mut FileContext,
        key: &PredicateKey,
        value: &str,
    ) -> Result<MatchResult> {
        // 1. Determine the language from the file extension.
        let extension = context
            .path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        let binding = profiles::list_language_profiles();
        let profile = match binding.iter().find(|p| p.extensions.contains(&extension)) {
            Some(p) => p,
            None => return Ok(MatchResult::Boolean(false)), // Not a supported language for this predicate.
        };

        // 2. Get the tree-sitter query string for the specific predicate.
        let ts_query_str = match profile.queries.get(key) {
            Some(q) if !q.is_empty() => q,
            _ => return Ok(MatchResult::Boolean(false)), // This predicate is not implemented for this language yet.
        };

        // 3. Get content and lazily get the parsed tree from the file context.
        let content = context.get_content()?.to_string(); // Clone to avoid borrow issues
        let tree = match context.get_tree(profile.language.clone()) {
            Ok(tree) => tree,
            Err(e) => {
                eprintln!(
                    "Warning: Failed to parse {}: {}. Skipping.",
                    context.path.display(),
                    e
                );
                return Ok(MatchResult::Boolean(false));
            }
        };

        // 4. Compile the tree-sitter query.
        let query = Query::new(&profile.language, ts_query_str)
            .with_context(|| format!("Failed to compile tree-sitter query for key {key:?}"))?;
        let mut cursor = QueryCursor::new();
        let mut ranges = Vec::new();

        // 5. Execute the query and check for a match.
        let captures = cursor.matches(&query, tree.root_node(), content.as_bytes());

        for m in captures {
            for capture in m.captures {
                // We only care about nodes captured with the name `@match`.
                let capture_name = &query.capture_names()[capture.index as usize];
                if *capture_name != "match" {
                    continue;
                }

                let captured_node = capture.node;
                let captured_text = captured_node.utf8_text(content.as_bytes())?;

                // Use the correct matching strategy based on the predicate type.
                let is_match = match key {
                    // Content-based predicates check for substrings.
                    PredicateKey::Import | PredicateKey::Comment | PredicateKey::Str => {
                        captured_text.contains(value)
                    }
                    // Definition-based predicates require an exact match on the identifier, unless a wildcard is used.
                    _ => value == "." || captured_text == value,
                };

                if is_match {
                    ranges.push(captured_node.range());
                }
            }
        }

        Ok(MatchResult::Hunks(ranges))
    }
}
