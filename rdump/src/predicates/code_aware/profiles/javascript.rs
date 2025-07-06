use super::LanguageProfile;
use crate::parser::PredicateKey;
use std::collections::HashMap;

/// Creates the profile for the JavaScript language.
pub(super) fn create_javascript_profile() -> LanguageProfile {
    let language = tree_sitter_javascript::language();
    let mut queries = HashMap::new();

    queries.insert(
        PredicateKey::Def,
        "(class_declaration name: (identifier) @match)".to_string(),
    );
    queries.insert(PredicateKey::Func, "[ (function_declaration name: (identifier) @match) (method_definition name: (property_identifier) @match) ]".to_string());
    queries.insert(
        PredicateKey::Import,
        "(import_statement) @match".to_string(),
    );

    LanguageProfile { language, queries }
}
