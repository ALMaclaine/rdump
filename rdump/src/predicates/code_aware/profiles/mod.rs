use crate::parser::PredicateKey;
use once_cell::sync::Lazy;
use std::collections::HashMap;

mod go;
mod java;
mod javascript;
mod python;
mod react; // Add react module
mod rust;
mod typescript;

/// Defines the tree-sitter queries and metadata for a specific language.
pub struct LanguageProfile {
    pub name: &'static str,
    pub extensions: Vec<&'static str>,
    pub(super) language: tree_sitter::Language,
    pub queries: HashMap<PredicateKey, String>,
}

pub(super) static LANGUAGE_PROFILES: Lazy<HashMap<&'static str, LanguageProfile>> =
    Lazy::new(|| {
        let mut m = HashMap::new();
        m.insert("rs", rust::create_rust_profile());
        m.insert("py", python::create_python_profile());
        m.insert("go", go::create_go_profile());
        m.insert("java", java::create_java_profile());
        m.insert("ts", typescript::create_typescript_profile());
        m.insert("js", javascript::create_javascript_profile());
        m.insert("jsx", react::create_react_profile());
        m
    });

/// Returns a list of all configured language profiles.
pub fn list_language_profiles() -> Vec<&'static LanguageProfile> {
    LANGUAGE_PROFILES.values().collect()
}
