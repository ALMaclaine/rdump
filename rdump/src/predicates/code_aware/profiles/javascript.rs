use super::LanguageProfile;
use crate::parser::PredicateKey;
use std::collections::HashMap;

/// Creates the profile for the JavaScript language.
pub(super) fn create_javascript_profile() -> LanguageProfile {
    let language = tree_sitter_javascript::language();
    let mut queries = HashMap::new();

    let class_query = "(class_declaration name: (identifier) @match)";
    let func_query = "[ (function_declaration name: (identifier) @match) (method_definition name: (property_identifier) @match) ]";

    queries.insert(PredicateKey::Def, [class_query, func_query].join("\n"));
    queries.insert(PredicateKey::Class, class_query.to_string());
    queries.insert(PredicateKey::Func, func_query.to_string());

    queries.insert(
        PredicateKey::Import,
        "(import_statement) @match".to_string(),
    );
    queries.insert(
       PredicateKey::Call,
       "[ (call_expression function: [ (identifier) @match (member_expression property: (property_identifier) @match) ]) (new_expression constructor: (identifier) @match) ]".to_string()
   );

    queries.insert(
        PredicateKey::Comment,
        "[(comment) @match (regex) @match]".to_string(),
    ); // JS Regexes are basically comments
    queries.insert(
        PredicateKey::Str,
        "[(string) @match (template_string) @match]".to_string(),
    );

    // --- React Hook Queries ---
    let hook_query = "
        (call_expression
            function: (identifier) @match
            (#match? @match \"^(use)\")
        )
    ";
    let custom_hook_query = r#"
[
  (function_declaration
    name: (identifier) @match)
  (lexical_declaration
    (variable_declarator
      name: (identifier) @match
      value: (arrow_function)))
  (export_statement
    declaration: [
      (function_declaration
        name: (identifier) @match)
      (lexical_declaration
        (variable_declarator
          name: (identifier) @match
          value: (arrow_function)))
    ])
]
(#match? @match "^use[A-Z]")
"#;
    queries.insert(PredicateKey::Hook, hook_query.to_string());
    queries.insert(PredicateKey::CustomHook, custom_hook_query.to_string());

    LanguageProfile {
        name: "JavaScript",
        extensions: vec!["js"],
        language,
        queries,
    }
}
