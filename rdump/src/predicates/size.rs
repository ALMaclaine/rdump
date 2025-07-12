use super::{helpers, PredicateEvaluator};
use crate::evaluator::{FileContext, MatchResult};
use crate::parser::PredicateKey;
use anyhow::Result;

pub(super) struct SizeEvaluator;
impl PredicateEvaluator for SizeEvaluator {
    fn evaluate(
        &self,
        context: &mut FileContext,
        _key: &PredicateKey,
        value: &str,
    ) -> Result<MatchResult> {
        let metadata = context.path.metadata()?;
        let file_size = metadata.len();
        Ok(MatchResult::Boolean(helpers::parse_and_compare_size(
            file_size, value,
        )?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use std::path::PathBuf;
    use tempfile::tempdir;

    #[test]
    fn test_size_evaluator_valid_comparisons() -> Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("file_1kb");
        let mut file = File::create(&file_path)?;
        file.write_all(&[0; 1024])?;

        let mut context = FileContext::new(file_path, PathBuf::from("/"));
        let evaluator = SizeEvaluator;

        // Exact match
        assert!(evaluator
            .evaluate(&mut context, &PredicateKey::Size, "=1kb")?
            .is_match());
        assert!(evaluator
            .evaluate(&mut context, &PredicateKey::Size, "=1024")?
            .is_match());

        // Greater than
        assert!(evaluator
            .evaluate(&mut context, &PredicateKey::Size, ">1000")?
            .is_match());
        assert!(evaluator
            .evaluate(&mut context, &PredicateKey::Size, ">0.9kb")?
            .is_match());
        assert!(!evaluator
            .evaluate(&mut context, &PredicateKey::Size, ">2kb")?
            .is_match());

        // Less than
        assert!(evaluator
            .evaluate(&mut context, &PredicateKey::Size, "<2kb")?
            .is_match());
        assert!(!evaluator
            .evaluate(&mut context, &PredicateKey::Size, "<1kb")?
            .is_match());

        Ok(())
    }

    #[test]
    fn test_size_evaluator_empty_file() -> Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("empty_file");
        File::create(&file_path)?;

        let mut context = FileContext::new(file_path, PathBuf::from("/"));
        let evaluator = SizeEvaluator;

        assert!(evaluator
            .evaluate(&mut context, &PredicateKey::Size, "=0")?
            .is_match());
        assert!(evaluator
            .evaluate(&mut context, &PredicateKey::Size, "<1")?
            .is_match());
        assert!(!evaluator
            .evaluate(&mut context, &PredicateKey::Size, ">0")?
            .is_match());

        Ok(())
    }

    #[test]
    fn test_size_evaluator_invalid_input() -> Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("any_file");
        File::create(&file_path)?;
        let mut context = FileContext::new(file_path, PathBuf::from("/"));
        let evaluator = SizeEvaluator;

        // Invalid number
        assert!(evaluator
            .evaluate(&mut context, &PredicateKey::Size, ">abc")
            .is_err());

        // Invalid operator
        assert!(evaluator
            .evaluate(&mut context, &PredicateKey::Size, "?123")
            .is_err());
            
        // Missing value
        assert!(evaluator
            .evaluate(&mut context, &PredicateKey::Size, ">")
            .is_err());

        Ok(())
    }
}
