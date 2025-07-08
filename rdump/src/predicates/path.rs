use super::PredicateEvaluator;
use crate::evaluator::{FileContext, MatchResult};
use crate::parser::PredicateKey;
use anyhow::Result;
use glob::Pattern;

pub(super) struct PathEvaluator;
impl PredicateEvaluator for PathEvaluator {
    fn evaluate(
        &self,
        context: &mut FileContext,
        _key: &PredicateKey,
        value: &str,
    ) -> Result<MatchResult> {
        let path_str = context.path.to_string_lossy();

        if value.contains('*') || value.contains('?') || value.contains('[') || value.contains('{')
        {
            let pattern = Pattern::new(value)?;
            Ok(MatchResult::Boolean(pattern.matches(&path_str)))
        } else {
            Ok(MatchResult::Boolean(path_str.contains(value)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_path_evaluator_contains() {
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

    #[test]
    fn test_path_evaluator_glob() {
        let mut context = FileContext::new(PathBuf::from("/home/user/project/src/main.rs"));
        let evaluator = PathEvaluator;

        // This will match because it's a full path glob
        assert!(evaluator
            .evaluate(
                &mut context,
                &PredicateKey::Path,
                "/home/user/project/src/main.rs"
            )
            .unwrap()
            .is_match());

        // This will match
        assert!(evaluator
            .evaluate(&mut context, &PredicateKey::Path, "**/main.rs")
            .unwrap()
            .is_match());

        // This will match
        assert!(evaluator
            .evaluate(&mut context, &PredicateKey::Path, "/home/user/*/src/main.rs")
            .unwrap()
            .is_match());

        // This will NOT match because glob does not do substring matches
        assert!(!evaluator
            .evaluate(&mut context, &PredicateKey::Path, "src/*.rs")
            .unwrap()
            .is_match());

        // This is what the user would have to do for partials
        assert!(evaluator
            .evaluate(&mut context, &PredicateKey::Path, "**/src/*.rs")
            .unwrap()
            .is_match());
    }

    #[test]
    fn test_more_glob_patterns() {
        let mut context = FileContext::new(PathBuf::from("/home/user/project/src/main.rs"));
        let evaluator = PathEvaluator;

        // Test '?' wildcard
        assert!(evaluator
            .evaluate(&mut context, &PredicateKey::Path, "**/main.??")
            .unwrap()
            .is_match());
        assert!(evaluator
            .evaluate(&mut context, &PredicateKey::Path, "**/main.r?")
            .unwrap()
            .is_match());

        // Test '[]' wildcard
        assert!(!evaluator
            .evaluate(&mut context, &PredicateKey::Path, "**/*.[rs]")
            .unwrap()
            .is_match());
        assert!(evaluator
            .evaluate(&mut context, &PredicateKey::Path, "**/*.[a-z]s")
            .unwrap()
            .is_match());
        assert!(!evaluator
            .evaluate(&mut context, &PredicateKey::Path, "**/*.[ts]")
            .unwrap()
            .is_match());
    }

    #[test]
    fn test_invalid_glob_pattern() {
        let mut context = FileContext::new(PathBuf::from("/home/user/project/src/main.rs"));
        let evaluator = PathEvaluator;

        // Test invalid glob pattern
        assert!(evaluator
            .evaluate(&mut context, &PredicateKey::Path, "**/[main.rs")
            .is_err());
    }

    #[test]
    fn test_empty_path_query() {
        let mut context = FileContext::new(PathBuf::from("/home/user/project/src/main.rs"));
        let evaluator = PathEvaluator;

        // Empty string should match everything with `contains`
        assert!(evaluator
            .evaluate(&mut context, &PredicateKey::Path, "")
            .unwrap()
            .is_match());
    }
}
