use super::LanguageProfile;
use crate::parser::PredicateKey;
use std::collections::HashMap;

/// Creates the profile for the Java language.
pub(super) fn create_java_profile() -> LanguageProfile {
    let language = tree_sitter_java::language();
    let mut queries = HashMap::new();

    // --- Definitions ---
    let class_query = "(class_declaration name: (identifier) @match)";
    let interface_query = "(interface_declaration name: (identifier) @match)";
    let enum_query = "(enum_declaration name: (identifier) @match)";

    queries.insert(PredicateKey::Def, format!("[ {} {} {} ]", class_query, interface_query, enum_query));
    queries.insert(PredicateKey::Class, class_query.to_string());
    queries.insert(PredicateKey::Interface, interface_query.to_string());
    queries.insert(PredicateKey::Enum, enum_query.to_string());

    // --- Functions & Calls ---
    queries.insert(PredicateKey::Func, "[ (method_declaration name: (identifier) @match) (constructor_declaration name: (identifier) @match) ]".to_string());
    queries.insert(PredicateKey::Call, "[ (method_invocation name: (identifier) @match) (object_creation_expression type: (type_identifier) @match) ]".to_string());

    // --- Other ---
    queries.insert(PredicateKey::Import, "(import_declaration) @match".to_string());
    queries.insert(PredicateKey::Comment, "[(line_comment) @match (block_comment) @match]".to_string());
    queries.insert(PredicateKey::Str, "(string_literal) @match".to_string());

    LanguageProfile {
        name: "Java",
        extensions: vec!["java"],
        language,
        queries,
    }
}
