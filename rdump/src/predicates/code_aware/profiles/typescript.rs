use super::LanguageProfile;
use crate::parser::PredicateKey;
use std::collections::HashMap;

/// Creates the profile for the TypeScript language.
pub(super) fn create_typescript_profile() -> LanguageProfile {
    let language = tree_sitter_typescript::language_typescript();
    let mut queries = HashMap::new();

    let class_query = "(class_declaration name: (type_identifier) @match)";
    let interface_query = "(interface_declaration name: (type_identifier) @match)";
    let type_query = "(type_alias_declaration name: (type_identifier) @match)";
    let enum_query = "(enum_declaration name: (identifier) @match)";

    let def_query = [class_query, interface_query, type_query, enum_query].join("\n");
    queries.insert(PredicateKey::Def, def_query);

    queries.insert(PredicateKey::Class, class_query.to_string());
    queries.insert(PredicateKey::Interface, interface_query.to_string());
    queries.insert(PredicateKey::Type, type_query.to_string());
    queries.insert(PredicateKey::Enum, enum_query.to_string());

    queries.insert(PredicateKey::Func, "[ (function_declaration name: (identifier) @match) (method_definition name: (property_identifier) @match) ]".to_string());
    queries.insert(
        PredicateKey::Import,
        "(import_statement) @match".to_string(),
    );
    queries.insert(
       PredicateKey::Call,
       "[ (call_expression function: [ (identifier) @match (member_expression property: (property_identifier) @match) ]) (new_expression constructor: [ (identifier) @match (type_identifier) @match ]) ]".to_string()
   );

    queries.insert(PredicateKey::Comment, "(comment) @match".to_string());
    queries.insert(
        PredicateKey::Str,
        "[(string) @match (template_string) @match]".to_string(),
    );

    LanguageProfile {
        name: "TypeScript",
        extensions: vec!["ts"],
        language,
        queries,
    }
}
