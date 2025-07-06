use super::PredicateEvaluator;
use crate::evaluator::FileContext;
use crate::parser::PredicateKey;
use anyhow::Result;

pub(super) struct ExtEvaluator;
impl PredicateEvaluator for ExtEvaluator {
    fn evaluate(
        &self,
        context: &mut FileContext,
        _key: &PredicateKey,
        value: &str,
    ) -> Result<bool> {
        let file_ext = context
            .path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        Ok(file_ext.eq_ignore_ascii_case(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_ext_evaluator() {
        let mut context_rs = FileContext::new(PathBuf::from("main.rs"));
        let mut context_toml = FileContext::new(PathBuf::from("Cargo.TOML"));
        let mut context_no_ext = FileContext::new(PathBuf::from("README"));
        let mut context_dotfile = FileContext::new(PathBuf::from(".bashrc"));

        let evaluator = ExtEvaluator;
        assert!(evaluator
            .evaluate(&mut context_rs, &PredicateKey::Ext, "rs")
            .unwrap());
        assert!(!evaluator
            .evaluate(&mut context_rs, &PredicateKey::Ext, "toml")
            .unwrap());
        assert!(
            evaluator
                .evaluate(&mut context_toml, &PredicateKey::Ext, "toml")
                .unwrap(),
            "Should be case-insensitive"
        );
        assert!(!evaluator
            .evaluate(&mut context_no_ext, &PredicateKey::Ext, "rs")
            .unwrap());
        assert!(
            !evaluator
                .evaluate(&mut context_dotfile, &PredicateKey::Ext, "bashrc")
                .unwrap(),
            "Dotfiles should have no extension"
        );
    }
}
