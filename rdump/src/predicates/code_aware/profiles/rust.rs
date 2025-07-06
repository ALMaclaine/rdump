use super::LanguageProfile;
use crate::parser::PredicateKey;
use std::collections::HashMap;

/// Creates the profile for the Rust language.
pub(super) fn create_rust_profile() -> LanguageProfile {
    let language = tree_sitter_rust::language();
    let mut queries = HashMap::new();

    let struct_query = "(struct_item name: (_) @match)";
    let enum_query = "(enum_item name: (_) @match)";
    let trait_query = "(trait_item name: (_) @match)";
    let type_query = "(type_item name: (type_identifier) @match)";

    let def_query = [struct_query, enum_query, trait_query, type_query].join("\n");

    queries.insert(PredicateKey::Def, def_query);
    queries.insert(PredicateKey::Struct, struct_query.to_string());
    queries.insert(PredicateKey::Enum, enum_query.to_string());
    queries.insert(PredicateKey::Trait, trait_query.to_string());
    queries.insert(PredicateKey::Type, type_query.to_string());

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

   // Query for function and method call sites.
   queries.insert(
       PredicateKey::Call,
       "
       (call_expression
           function: [
               (identifier) @match
               (field_expression field: (field_identifier) @match)
           ]
       )
       (macro_invocation macro: (identifier) @match)
       "
       .to_string(),
   );

    queries.insert(PredicateKey::Comment, "[(line_comment) @match (block_comment) @match]".to_string());
    queries.insert(PredicateKey::Str, "[(string_literal) @match (raw_string_literal) @match]".to_string());

    LanguageProfile {
        name: "Rust",
        extensions: vec!["rs"],
        language,
        queries,
    }
}