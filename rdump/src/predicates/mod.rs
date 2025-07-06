use crate::evaluator::FileContext;
use crate::parser::PredicateKey;
use anyhow::Result;
use std::collections::HashMap;

// The core trait that all predicate evaluators must implement.
pub trait PredicateEvaluator {
    fn evaluate(&self, context: &mut FileContext, value: &str) -> Result<bool>;
}

// --- Concrete Implementations ---

struct ExtEvaluator;
impl PredicateEvaluator for ExtEvaluator {
    fn evaluate(&self, context: &mut FileContext, value: &str) -> Result<bool> {
        let file_ext = context.path.extension().and_then(|s| s.to_str()).unwrap_or("");
        Ok(file_ext.eq_ignore_ascii_case(value))
    }
}

struct PathEvaluator;
impl PredicateEvaluator for PathEvaluator {
    fn evaluate(&self, context: &mut FileContext, value: &str) -> Result<bool> {
        let path_str = context.path.to_string_lossy();
        Ok(path_str.contains(value))
    }
}

struct NameEvaluator;
impl PredicateEvaluator for NameEvaluator {
    fn evaluate(&self, context: &mut FileContext, value: &str) -> Result<bool> {
        let file_name = context.path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        let pattern = glob::Pattern::new(value)?;
        Ok(pattern.matches(file_name))
    }
}

struct ContainsEvaluator;
impl PredicateEvaluator for ContainsEvaluator {
    fn evaluate(&self, context: &mut FileContext, value: &str) -> Result<bool> {
        let content = context.get_content()?;
        Ok(content.contains(value))
    }
}

struct MatchesEvaluator;
impl PredicateEvaluator for MatchesEvaluator {
    fn evaluate(&self, context: &mut FileContext, value: &str) -> Result<bool> {
        let content = context.get_content()?;
        let re = regex::Regex::new(value)?;
        Ok(re.is_match(content))
    }
}

// ... We will add Size, Modified, and Def evaluators here later ...

// The "Registry" that holds all our evaluators.
pub fn create_predicate_registry() -> HashMap<PredicateKey, Box<dyn PredicateEvaluator + Send + Sync>> {
    let mut registry: HashMap<PredicateKey, Box<dyn PredicateEvaluator + Send + Sync>> = HashMap::new();

    // The `Send + Sync` bounds are required because we use this in a multi-threaded
    // context with Rayon. All our current evaluators are safe.
    registry.insert(PredicateKey::Ext, Box::new(ExtEvaluator));
    registry.insert(PredicateKey::Path, Box::new(PathEvaluator));
    registry.insert(PredicateKey::Name, Box::new(NameEvaluator));
    registry.insert(PredicateKey::Contains, Box::new(ContainsEvaluator));
    registry.insert(PredicateKey::Matches, Box::new(MatchesEvaluator));

    registry
}
