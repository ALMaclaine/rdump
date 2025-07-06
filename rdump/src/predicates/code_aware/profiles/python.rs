use super::LanguageProfile;
use crate::parser::PredicateKey;
use std::collections::HashMap;

/// Creates the profile for the Python language.
pub(super) fn create_python_profile() -> LanguageProfile {
    let language = tree_sitter_python::language();
    let mut queries = HashMap::new();

    let class_def_query = "(class_definition name: (identifier) @match)";
   queries.insert(PredicateKey::Def, class_def_query.to_string());
   queries.insert(PredicateKey::Class, class_def_query.to_string());

    // Query for function definitions.
    queries.insert(
        PredicateKey::Func,
        "
        (function_definition name: (identifier) @match)
        "
        .to_string(),
    );

    // Query for `import` and `from ... import` statements.
    queries.insert(
        PredicateKey::Import,
        "
        [
            (import_statement) @match
            (import_from_statement) @match
        ]
        "
        .to_string(),
    );

   queries.insert(PredicateKey::Comment, "(comment) @match".to_string());
   queries.insert(PredicateKey::Str, "(string) @match".to_string());

    LanguageProfile { language, queries }
}
