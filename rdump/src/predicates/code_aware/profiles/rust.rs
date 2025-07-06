use super::LanguageProfile;
use crate::parser::PredicateKey;
use std::collections::HashMap;

/// Creates the profile for the Rust language.
pub(super) fn create_rust_profile() -> LanguageProfile {
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
