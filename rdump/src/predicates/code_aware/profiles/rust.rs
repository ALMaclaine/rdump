use super::LanguageProfile;
use crate::parser::PredicateKey;
use std::collections::HashMap;

/// Creates the profile for the Rust language.
pub(super) fn create_rust_profile() -> LanguageProfile {
    let language = tree_sitter_rust::language();
    let mut queries = HashMap::new();

    let def_query = r#"
        (struct_item name: (_) @match)
        (enum_item name: (_) @match)
        (trait_item name: (_) @match)
        "#;

    queries.insert(
        PredicateKey::Def,
        def_query.to_string(),
    );
   queries.insert(PredicateKey::Struct, "(struct_item name: (_) @match)".to_string());
   queries.insert(PredicateKey::Enum, "(enum_item name: (_) @match)".to_string());
   queries.insert(PredicateKey::Trait, "(trait_item name: (_) @match)".to_string());
   queries.insert(PredicateKey::Type, "(type_item name: (type_identifier) @match)".to_string());

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

    LanguageProfile { language, queries }
}
