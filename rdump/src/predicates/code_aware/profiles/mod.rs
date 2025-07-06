use crate::parser::PredicateKey;
use lazy_static::lazy_static;
use std::collections::HashMap;

mod javascript;
mod python;
mod rust;
mod typescript;
mod go;

/// Defines the tree-sitter queries for a specific language.
pub(super) struct LanguageProfile {
    pub(super) language: tree_sitter::Language,
    pub(super) queries: HashMap<PredicateKey, String>,
}

lazy_static! {
    pub(super) static ref LANGUAGE_PROFILES: HashMap<&'static str, LanguageProfile> = {
        let mut m = HashMap::new();
        m.insert("rs", rust::create_rust_profile());
        m.insert("py", python::create_python_profile());
        m.insert("go", go::create_go_profile());
        let ts_profile = typescript::create_typescript_profile();
        m.insert("ts", ts_profile);
        m.insert("tsx", typescript::create_typescript_profile());
        let js_profile = javascript::create_javascript_profile();
        m.insert("js", js_profile);
        m.insert("jsx", javascript::create_javascript_profile());
        m
    };
}
