use super::LanguageProfile;
use crate::parser::PredicateKey;
use std::collections::HashMap;

/// Creates the profile for the React (JSX/TSX).
pub(super) fn create_react_profile() -> LanguageProfile {
    let language = tree_sitter_typescript::language_tsx();
    let mut queries = HashMap::new();

    // --- Component & Element Queries ---
    let component_query = "
        [
            (class_declaration name: (type_identifier) @match)
            (function_declaration name: (identifier) @match)
            (lexical_declaration
                (variable_declarator
                    name: (identifier) @match
                    value: (arrow_function)
                )
            )
            (export_statement
                declaration: (lexical_declaration
                    (variable_declarator
                        name: (identifier) @match
                        value: (call_expression
                            function: (member_expression
                                property: (property_identifier) @_prop
                            )
                            (#eq? @_prop \"memo\")
                        )
                    )
                )
            )
            (lexical_declaration
                (variable_declarator
                    name: (identifier) @match
                    value: (call_expression
                        function: (member_expression
                            property: (property_identifier) @_prop
                        )
                        (#eq? @_prop \"memo\")
                    )
                )
            )
        ]
    ";
    let element_query = "
        [
            (jsx_opening_element name: (_) @match)
            (jsx_self_closing_element name: (_) @match)
        ]
    ";
    queries.insert(PredicateKey::Component, component_query.to_string());
    queries.insert(PredicateKey::Element, element_query.to_string());

    // --- Hook Queries ---
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

    // --- Prop Query ---
    let prop_query = "(jsx_attribute (property_identifier) @match)";
    queries.insert(PredicateKey::Prop, prop_query.to_string());

    // --- Generic Queries (reusing from TS) ---
    queries.insert(
        PredicateKey::Import,
        "(import_statement) @match".to_string(),
    );
    queries.insert(PredicateKey::Comment, "(comment) @match".to_string());
    queries.insert(
        PredicateKey::Str,
        "[(string) @match (template_string) @match]".to_string(),
    );

    LanguageProfile {
        name: "React",
        extensions: vec!["jsx", "tsx"],
        language,
        queries,
    }
}
