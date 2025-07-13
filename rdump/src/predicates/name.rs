use super::PredicateEvaluator;
use crate::evaluator::{FileContext, MatchResult};
use crate::parser::PredicateKey;
use anyhow::{anyhow, Result};
use glob::{MatchOptions, Pattern};

pub(super) struct NameEvaluator;
impl PredicateEvaluator for NameEvaluator {
    fn evaluate(
        &self,
        context: &mut FileContext,
        _key: &PredicateKey,
        value: &str,
    ) -> Result<MatchResult> {
        if value.is_empty() {
            return Err(anyhow!("Invalid glob pattern: cannot be empty."));
        }

        let file_name = context
            .path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        let options = MatchOptions {
            case_sensitive: false,
            ..Default::default()
        };
        let pattern = Pattern::new(value)?;
        Ok(MatchResult::Boolean(
            pattern.matches_with(file_name, options),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_name_evaluator() {
        let mut context1 = FileContext::new(PathBuf::from("/home/user/Cargo.toml"), PathBuf::from("/"));
        let mut context2 = FileContext::new(PathBuf::from("/home/user/main.rs"), PathBuf::from("/"));

        let evaluator = NameEvaluator;
        assert!(evaluator
            .evaluate(&mut context1, &PredicateKey::Name, "Cargo.toml")
            .unwrap()
            .is_match());
        assert!(
            evaluator
                .evaluate(&mut context1, &PredicateKey::Name, "C*.toml")
                .unwrap()
                .is_match(),
            "Glob pattern should match"
        );
        assert!(
            evaluator
                .evaluate(&mut context2, &PredicateKey::Name, "*.rs")
                .unwrap()
                .is_match(),
            "Glob pattern should match"
        );
        assert!(!evaluator
            .evaluate(&mut context1, &PredicateKey::Name, "*.rs")
            .unwrap()
            .is_match());
    }

    #[test]
    fn test_name_evaluator_case_insensitive() {
        let mut context =
            FileContext::new(PathBuf::from("/home/user/MyFile.txt"), PathBuf::from("/"));
        let evaluator = NameEvaluator;
        assert!(evaluator
            .evaluate(&mut context, &PredicateKey::Name, "myfile.txt")
            .unwrap()
            .is_match());
        assert!(evaluator
            .evaluate(&mut context, &PredicateKey::Name, "MYFILE.TXT")
            .unwrap()
            .is_match());
    }
}
