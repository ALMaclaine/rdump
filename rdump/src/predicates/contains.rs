use super::PredicateEvaluator;
use crate::evaluator::{FileContext, MatchResult};
use crate::parser::PredicateKey;
use anyhow::Result;

pub(super) struct ContainsEvaluator;

impl PredicateEvaluator for ContainsEvaluator {
    fn evaluate(
        &self,
        context: &mut FileContext,
        _key: &PredicateKey,
        value: &str,
    ) -> Result<MatchResult> {
        let content = context.get_content()?;
        Ok(MatchResult::Boolean(content.contains(value)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;
    fn create_temp_file(content: &str) -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "{}", content).unwrap();
        file
    }
    #[test]
    fn test_contains_evaluator() {
        let file = create_temp_file("Hello world\nThis is a test.");
        let mut context = FileContext::new(file.path().to_path_buf());
        let evaluator = ContainsEvaluator;
        assert!(evaluator
            .evaluate(&mut context, &PredicateKey::Contains, "world")
            .unwrap()
            .is_match());
        assert!(evaluator
            .evaluate(&mut context, &PredicateKey::Contains, "is a test")
            .unwrap()
            .is_match());
        assert!(!evaluator
            .evaluate(&mut context, &PredicateKey::Contains, "goodbye")
            .unwrap()
            .is_match());
    }
}
