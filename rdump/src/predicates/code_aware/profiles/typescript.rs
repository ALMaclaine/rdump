use super::LanguageProfile;
use crate::parser::PredicateKey;
use std::collections::HashMap;

/// Creates the profile for the TypeScript language.
pub(super) fn create_typescript_profile() -> LanguageProfile {
    let language = tree_sitter_typescript::language_typescript();
    let mut queries = HashMap::new();

    queries.insert(PredicateKey::Def, "[ (class_declaration name: (type_identifier) @match) (interface_declaration name: (type_identifier) @match) (type_alias_declaration name: (type_identifier) @match) ]".to_string());
    queries.insert(PredicateKey::Func, "[ (function_declaration name: (identifier) @match) (method_definition name: (property_identifier) @match) ]".to_string());
    queries.insert(
        PredicateKey::Import,
        "(import_statement) @match".to_string(),
    );

    LanguageProfile { language, queries }
}
