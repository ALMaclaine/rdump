use super::LanguageProfile;
use crate::parser::PredicateKey;
use std::collections::HashMap;

/// Creates the profile for the Go language.
pub(super) fn create_go_profile() -> LanguageProfile {
    let language = tree_sitter_go::language();
    let mut queries = HashMap::new();

    // --- Definitions ---
    let struct_query = "(type_declaration (type_spec name: (type_identifier) @match type: (struct_type)))";
    let interface_query = "(type_declaration (type_spec name: (type_identifier) @match type: (interface_type)))";

    queries.insert(PredicateKey::Def, format!("[ {} {} ]", struct_query, interface_query));
    queries.insert(PredicateKey::Struct, struct_query.to_string());
    queries.insert(PredicateKey::Interface, interface_query.to_string());
    queries.insert(PredicateKey::Type, "(type_declaration (type_spec name: (type_identifier) @match))".to_string());

    // --- Functions & Calls ---
    queries.insert(PredicateKey::Func, "[ (function_declaration name: (identifier) @match) (method_declaration name: (field_identifier) @match) ]".to_string());
    queries.insert(PredicateKey::Call, "(call_expression function: [ (identifier) @match (selector_expression field: (field_identifier) @match) ])".to_string());

    // --- Other ---
    queries.insert(PredicateKey::Import, "(import_declaration) @match".to_string());
    queries.insert(PredicateKey::Comment, "(comment) @match".to_string());
    queries.insert(PredicateKey::Str, "[ (interpreted_string_literal) @match (raw_string_literal) @match ]".to_string());

    LanguageProfile { language, queries }
}
