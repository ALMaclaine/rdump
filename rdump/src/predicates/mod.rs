pub mod code_aware;
pub mod contains;
pub mod ext;
mod helpers;
pub mod matches;
pub mod modified;
pub mod name;
pub mod path;
pub mod size;

use self::code_aware::CodeAwareEvaluator;
use self::contains::ContainsEvaluator;
use self::ext::ExtEvaluator;
use self::matches::MatchesEvaluator;
use self::modified::ModifiedEvaluator;
use self::name::NameEvaluator;
use self::path::PathEvaluator;
use self::size::SizeEvaluator;
use crate::evaluator::FileContext;
use crate::parser::PredicateKey;
use anyhow::Result;
use std::collections::HashMap;

// The core trait that all predicate evaluators must implement.
pub trait PredicateEvaluator {
    // The key is now passed to allow one evaluator to handle multiple predicate types.
    fn evaluate(&self, context: &mut FileContext, key: &PredicateKey, value: &str) -> Result<bool>;
}

/// Creates and populates the predicate registry.
pub fn create_predicate_registry(
) -> HashMap<PredicateKey, Box<dyn PredicateEvaluator + Send + Sync>> {
    let mut registry: HashMap<PredicateKey, Box<dyn PredicateEvaluator + Send + Sync>> =
        HashMap::new();

    registry.insert(PredicateKey::Ext, Box::new(ExtEvaluator));
    registry.insert(PredicateKey::Name, Box::new(NameEvaluator));
    registry.insert(PredicateKey::Path, Box::new(PathEvaluator));
    registry.insert(PredicateKey::Contains, Box::new(ContainsEvaluator));
    registry.insert(PredicateKey::Matches, Box::new(MatchesEvaluator));
    registry.insert(PredicateKey::Size, Box::new(SizeEvaluator));
    registry.insert(PredicateKey::Modified, Box::new(ModifiedEvaluator));

    // Register the single CodeAwareEvaluator for all semantic predicate keys.
    // It's a stateless struct, so creating multiple boxes is cheap.
    registry.insert(PredicateKey::Def, Box::new(CodeAwareEvaluator));
    registry.insert(PredicateKey::Func, Box::new(CodeAwareEvaluator));
    registry.insert(PredicateKey::Import, Box::new(CodeAwareEvaluator));

    registry
}
