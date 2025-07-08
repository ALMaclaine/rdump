use anyhow::Result;
use std::path::PathBuf;

use super::PredicateEvaluator;
use crate::evaluator::{FileContext, MatchResult};
use crate::parser::PredicateKey;

pub(super) struct InPathEvaluator;

impl PredicateEvaluator for InPathEvaluator {
    fn evaluate(
        &self,
        context: &mut FileContext,
        _key: &PredicateKey,
        value: &str,
    ) -> Result<MatchResult> {
        // The `value` is the directory path provided by the user.
        // We need to resolve it to an absolute path to perform a reliable check.
        let target_dir = PathBuf::from(value);
        let absolute_target_dir = if target_dir.is_absolute() {
            target_dir
        } else {
            // If the user provides a relative path, it's relative to the current working dir.
            // We need to get the CWD from the context or environment.
            // For now, let's assume we can get it from the context's root.
            // This might need adjustment based on how the evaluator's context is built.
            let root = &context.root;
            root.join(target_dir)
        };

        // Normalize both paths to handle `.` and `..` components.
        let absolute_target_dir = dunce::canonicalize(&absolute_target_dir)?;
        let file_path = &context.path;

        Ok(MatchResult::Boolean(
            file_path.starts_with(&absolute_target_dir),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_in_path_evaluator() -> Result<()> {
        let evaluator = InPathEvaluator;

        // Create a temporary directory structure
        let root_dir = tempdir()?;
        let root_path = root_dir.path();

        let project_dir = root_path.join("project");
        let src_dir = project_dir.join("src");
        let other_project_dir = root_path.join("other_project");
        fs::create_dir_all(&src_dir)?;
        fs::create_dir_all(&other_project_dir)?;

        let main_rs_path = src_dir.join("main.rs");
        fs::write(&main_rs_path, "fn main() {}")?;

        // --- Test Cases ---

        let mut context = FileContext::new(main_rs_path.clone(), root_path.to_path_buf());

        // 1. Absolute Path: Exact parent directory
        assert!(evaluator
            .evaluate(&mut context, &PredicateKey::In, src_dir.to_str().unwrap())?
            .is_match());

        // 2. Absolute Path: Grandparent directory
        assert!(evaluator
            .evaluate(
                &mut context,
                &PredicateKey::In,
                project_dir.to_str().unwrap()
            )?
            .is_match());

        // 3. Absolute Path: Non-matching directory
        assert!(!evaluator
            .evaluate(
                &mut context,
                &PredicateKey::In,
                other_project_dir.to_str().unwrap()
            )?
            .is_match());

        // 4. Relative Path: from the root
        assert!(evaluator
            .evaluate(&mut context, &PredicateKey::In, "project/src")?
            .is_match());

        // 5. Relative Path: with dot-slash
        assert!(evaluator
            .evaluate(&mut context, &PredicateKey::In, "./project/src")?
            .is_match());

        // 6. Relative Path: non-matching
        assert!(!evaluator
            .evaluate(&mut context, &PredicateKey::In, "other_project")?
            .is_match());
            
        // 7. A file is considered to be "in" the directory represented by its own path
        assert!(evaluator
            .evaluate(&mut context, &PredicateKey::In, main_rs_path.to_str().unwrap())?
            .is_match());

        Ok(())
    }
}

