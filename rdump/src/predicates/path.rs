use super::PredicateEvaluator;
use crate::evaluator::FileContext;
use crate::parser::PredicateKey;
use anyhow::Result;

pub(super) struct PathEvaluator;
impl PredicateEvaluator for PathEvaluator {
    fn evaluate(
        &self,
        context: &mut FileContext,
        _key: &PredicateKey,
        value: &str,
    ) -> Result<bool> {
        let path_str = context.path.to_string_lossy();
        Ok(path_str.contains(value))
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
            .unwrap());
        assert!(evaluator
            .evaluate(&mut context, &PredicateKey::Path, "/home/user")
            .unwrap());
        assert!(!evaluator
            .evaluate(&mut context, &PredicateKey::Path, "project/lib")
            .unwrap());
        assert!(evaluator
            .evaluate(&mut context, &PredicateKey::Path, "main.rs")
            .unwrap());
    }
}
