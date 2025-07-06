use super::{helpers, PredicateEvaluator};
use crate::evaluator::FileContext;
use crate::parser::PredicateKey;
use anyhow::Result;

pub(super) struct ModifiedEvaluator;
impl PredicateEvaluator for ModifiedEvaluator {
    fn evaluate(
        &self,
        context: &mut FileContext,
        _key: &PredicateKey,
        value: &str,
    ) -> Result<bool> {
        let metadata = context.path.metadata()?;
        let modified_time = metadata.modified()?;
        helpers::parse_and_compare_time(modified_time, value)
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
    fn test_modified_evaluator() {
        let file = create_temp_file("content");
        let mut context = FileContext::new(file.path().to_path_buf());

        let evaluator = ModifiedEvaluator;
        // File was just created
        assert!(evaluator
            .evaluate(&mut context, &PredicateKey::Modified, ">1m")
            .unwrap()); // Modified more recently than 1 min ago
        assert!(!evaluator
            .evaluate(&mut context, &PredicateKey::Modified, "<1m")
            .unwrap()); // Not modified longer than 1 min ago
    }
}
