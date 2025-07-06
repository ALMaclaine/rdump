use crate::evaluator::FileContext;
use crate::parser::PredicateKey;
use super::PredicateEvaluator;

use anyhow::{Context, Result};
use lazy_static::lazy_static;
use std::collections::HashMap;
use tree_sitter::{Query, QueryCursor};

/// Defines the tree-sitter queries for a specific language.
struct LanguageProfile {
    language: tree_sitter::Language,
    queries: HashMap<PredicateKey, String>,
}

// A static registry of language profiles, loaded at compile time.
lazy_static! {
    static ref LANGUAGE_PROFILES: HashMap<&'static str, LanguageProfile> = {
        let mut m = HashMap::new();
        // Phase 2.0: Only Rust is implemented.
        m.insert("rs", create_rust_profile());
        // In the future, we will add:
        // m.insert("py", create_python_profile());
        // m.insert("js", create_javascript_profile());
        m
    };
}

/// Creates the profile for the Rust language.
fn create_rust_profile() -> LanguageProfile {
    let language = tree_sitter_rust::language();
    let mut queries = HashMap::new();

    // Query for struct, enum, and trait definitions.
    // We capture the node associated with the name using `@match`.
    queries.insert(
        PredicateKey::Def,
        r#"
        (struct_item name: (_) @match)
        (enum_item name: (_) @match)
        (trait_item name: (_) @match)
        "#
        .to_string(),
    );

    // Query for standalone functions and methods in traits or impls.
    queries.insert(
        PredicateKey::Func,
        "
        [
            (function_item name: (identifier) @match)
            (function_signature_item name: (identifier) @match)
        ]"
        .to_string(),
    );
    // Query for the entire `use` declaration. We will match against its text content.
    queries.insert(
        PredicateKey::Import,
        "
        (use_declaration) @match
        "
        .to_string(),
    );

    LanguageProfile { language, queries }
}

/// The evaluator that uses tree-sitter to perform code-aware queries.
#[derive(Debug)]
pub struct CodeAwareEvaluator;

impl PredicateEvaluator for CodeAwareEvaluator {
    fn evaluate(&self, context: &mut FileContext, key: &PredicateKey, value: &str) -> Result<bool> {
        // 1. Determine the language from the file extension.
        let extension = context.path.extension().and_then(|s| s.to_str()).unwrap_or("");
        let profile = match LANGUAGE_PROFILES.get(extension) {
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