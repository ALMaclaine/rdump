use super::PredicateEvaluator;
use crate::evaluator::{FileContext, MatchResult};
use crate::parser::PredicateKey;
use anyhow::Result;

pub(super) struct PathEvaluator;
impl PredicateEvaluator for PathEvaluator {
    fn evaluate(
        &self,
        context: &mut FileContext,
        _key: &PredicateKey,
        value: &str,
    ) -> Result<MatchResult> {
        let path_str = context.path.to_string_lossy();
        Ok(MatchResult::Boolean(path_str.contains(value)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_path_evaluator() {
        let mut context = FileContext::new(PathBuf::from("/home/user/project/src/main.rs"));
        let evaluator = PathEvaluator;
        assert!(evaluator
            .evaluate(&mut context, &PredicateKey::Path, "project/src")
            .unwrap()
            .is_match());
        assert!(evaluator
            .evaluate(&mut context, &PredicateKey::Path, "/home/user")
            .unwrap()
            .is_match());
        assert!(!evaluator
            .evaluate(&mut context, &PredicateKey::Path, "project/lib")
            .unwrap()
            .is_match());
        assert!(evaluator
            .evaluate(&mut context, &PredicateKey::Path, "main.rs")
            .unwrap()
            .is_match());
    }
}
