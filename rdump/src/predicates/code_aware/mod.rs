use crate::evaluator::FileContext;
use crate::parser::PredicateKey;
use crate::predicates::PredicateEvaluator;
use anyhow::{Context, Result};
use tree_sitter::{Query, QueryCursor};

mod profiles;

/// The evaluator that uses tree-sitter to perform code-aware queries.
#[derive(Debug)]
pub struct CodeAwareEvaluator;

impl PredicateEvaluator for CodeAwareEvaluator {
    fn evaluate(&self, context: &mut FileContext, key: &PredicateKey, value: &str) -> Result<bool> {
        // 1. Determine the language from the file extension.
        let extension = context
            .path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        let profile = match profiles::LANGUAGE_PROFILES.get(extension) {
            Some(p) => p,
            None => return Ok(false), // Not a supported language for this predicate.
        };

        // 2. Get the tree-sitter query string for the specific predicate.
        let ts_query_str = match profile.queries.get(key) {
            Some(q) if !q.is_empty() => q,
            _ => return Ok(false), // This predicate is not implemented for this language yet.
        };

        // 3. Get content and lazily get the parsed tree from the file context.
        // We get content first to avoid mutable/immutable borrow issues with context.
        let content = context.get_content()?.to_string(); // Clone to avoid borrow issues
        let tree = context.get_tree(profile.language.clone())?;

        // 4. Compile the tree-sitter query.
        let query = Query::new(&profile.language, ts_query_str)
            .with_context(|| format!("Failed to compile tree-sitter query for key {:?}", key))?;
        let mut cursor = QueryCursor::new();

        // 5. Execute the query and check for a match.
        let captures = cursor.matches(&query, tree.root_node(), content.as_bytes());

        for m in captures {
            for capture in m.captures {
                // We only care about nodes captured with the name `@match`.
                let capture_name = &query.capture_names()[capture.index as usize];
                if *capture_name == "match" {
                    let captured_node = capture.node;
                    let captured_text = captured_node.utf8_text(content.as_bytes())?;

                    // `import:` uses substring matching, `def:` and `func:` use exact matching.
                    let is_match = if key == &PredicateKey::Import {
                        captured_text.contains(value)
                    } else {
                        captured_text == value
                    };

                    if is_match {
                        return Ok(true);
                    }
                }
            }
        }

        Ok(false)
    }
}
