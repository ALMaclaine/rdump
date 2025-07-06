use super::PredicateEvaluator;
use crate::evaluator::FileContext;
use crate::parser::PredicateKey;
use anyhow::Result;

pub(super) struct NameEvaluator;
impl PredicateEvaluator for NameEvaluator {
    fn evaluate(
        &self,
        context: &mut FileContext,
        _key: &PredicateKey,
        value: &str,
    ) -> Result<bool> {
        let file_name = context
            .path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        let pattern = glob::Pattern::new(value)?;
        Ok(pattern.matches(file_name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_name_evaluator() {
        let mut context1 = FileContext::new(PathBuf::from("/home/user/Cargo.toml"));
        let mut context2 = FileContext::new(PathBuf::from("/home/user/main.rs"));

        let evaluator = NameEvaluator;
        assert!(evaluator
            .evaluate(&mut context1, &PredicateKey::Name, "Cargo.toml")
            .unwrap());
        assert!(
            evaluator
                .evaluate(&mut context1, &PredicateKey::Name, "C*.toml")
                .unwrap(),
            "Glob pattern should match"
        );
        assert!(
            evaluator
                .evaluate(&mut context2, &PredicateKey::Name, "*.rs")
                .unwrap(),
            "Glob pattern should match"
        );
        assert!(!evaluator
            .evaluate(&mut context1, &PredicateKey::Name, "*.rs")
            .unwrap());
    }
}
