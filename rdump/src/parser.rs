use anyhow::{anyhow, Result};
use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "rql.pest"]
pub struct RqlParser;

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum PredicateKey {
    Ext,
    Name,
    Path,
    Contains,
    Matches,
    // --- NEW PREDICATES ---
    Size,
    Modified,
    // --- NEW SEMANTIC PREDICATES ---
    Def,
    Func,
    Import,
    // A key for testing or unknown predicates
    Other(String),
}

impl From<&str> for PredicateKey {
    fn from(s: &str) -> Self {
        match s {
            "ext" => Self::Ext,
            "name" => Self::Name,
            "path" => Self::Path,
            "contains" => Self::Contains,
            "matches" => Self::Matches,
            // --- NEW PREDICATES ---
            "size" => Self::Size,
            "modified" => Self::Modified,
            // --- NEW SEMANTIC PREDICATES ---
            "def" => Self::Def,
            "func" => Self::Func,
            "import" => Self::Import,
            // Any other key is captured here.
            other => Self::Other(other.to_string()),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum AstNode {
    Predicate(PredicateKey, String),
    LogicalOp(LogicalOperator, Box<AstNode>, Box<AstNode>),
    Not(Box<AstNode>),
}

#[derive(Debug, PartialEq)]
pub enum LogicalOperator {
    And,
    Or,
}

pub fn parse_query(query: &str) -> Result<AstNode> {
    let pairs = RqlParser::parse(Rule::query, query)?;
    build_ast_from_pairs(pairs.peek().unwrap())
}

fn build_ast_from_pairs(pair: Pair<Rule>) -> Result<AstNode> {
    match pair.as_rule() {
        Rule::query => build_ast_from_pairs(pair.into_inner().next().unwrap()),
        Rule::expression | Rule::logical_or | Rule::logical_and => build_ast_from_logical_op(pair),
        Rule::term => {
            let mut inner = pair.into_inner();
            let first = inner.next().unwrap();
            if first.as_rule() == Rule::NOT {
                let factor = inner.next().unwrap();
                let ast = build_ast_from_pairs(factor)?;
                Ok(AstNode::Not(Box::new(ast)))
            } else {
                build_ast_from_pairs(first)
            }
        }
        Rule::factor => build_ast_from_pairs(pair.into_inner().next().unwrap()),
        Rule::predicate => {
            let mut predicate_parts = pair.into_inner();
            let key_pair = predicate_parts.next().unwrap();
            let value_pair = predicate_parts.next().unwrap();
            let key = PredicateKey::from(key_pair.as_str());
            let value = unescape_value(value_pair.as_str());
            Ok(AstNode::Predicate(key, value))
        }
        _ => Err(anyhow!("Unknown rule: {:?}", pair.as_rule())),
    }
}

fn build_ast_from_logical_op(pair: Pair<Rule>) -> Result<AstNode> {
    let mut inner_pairs = pair.into_inner();
    let mut ast = build_ast_from_pairs(inner_pairs.next().unwrap())?;

    while let Some(op_pair) = inner_pairs.next() {
        let op = match op_pair.as_str() {
            "&" => LogicalOperator::And,
            "|" => LogicalOperator::Or,
            _ => unreachable!(),
        };
        let right_pair = inner_pairs.next().unwrap();
        let right_ast = build_ast_from_pairs(right_pair)?;
        ast = AstNode::LogicalOp(op, Box::new(ast), Box::new(right_ast));
    }
    Ok(ast)
}

fn unescape_value(value: &str) -> String {
    if value.starts_with('"') && value.ends_with('"') {
        return value[1..value.len() - 1].replace("\"", "\"");
    }
    if value.starts_with('\'') && value.ends_with('\'') {
        return value[1..value.len() - 1].replace("\\'", "\'");
    }
    value.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to create a predicate node for cleaner tests.
    fn predicate(key: PredicateKey, value: &str) -> Box<AstNode> {
        Box::new(AstNode::Predicate(key, value.to_string()))
    }

    #[test]
    fn test_parse_simple_predicate() {
        let ast = parse_query("ext:rs").unwrap();
        assert_eq!(ast, *predicate(PredicateKey::Ext, "rs"));
    }

    #[test]
    fn test_parse_predicate_with_quoted_value() {
        let ast = parse_query("name:\"foo bar\"").unwrap();
        assert_eq!(ast, *predicate(PredicateKey::Name, "foo bar"));
    }

    #[test]
    fn test_parse_logical_and() {
        let ast = parse_query("ext:rs & name:\"foo\"").unwrap();
        assert_eq!(
            ast,
            AstNode::LogicalOp(
                LogicalOperator::And,
                predicate(PredicateKey::Ext, "rs"),
                predicate(PredicateKey::Name, "foo")
            )
        );
    }

    #[test]
    fn test_parse_logical_or() {
        let ast = parse_query("ext:rs | ext:toml").unwrap();
        assert_eq!(
            ast,
            AstNode::LogicalOp(
                LogicalOperator::Or,
                predicate(PredicateKey::Ext, "rs"),
                predicate(PredicateKey::Ext, "toml")
            )
        );
    }

    #[test]
    fn test_parse_negation() {
        let ast = parse_query("!ext:rs").unwrap();
        assert_eq!(ast, AstNode::Not(predicate(PredicateKey::Ext, "rs")));
    }

    #[test]
    fn test_parse_complex_query() {
        let ast = parse_query("ext:rs & (name:\"foo\" | name:\"bar\") & !path:tests").unwrap();
        let inner_or = AstNode::LogicalOp(
            LogicalOperator::Or,
            predicate(PredicateKey::Name, "foo"),
            predicate(PredicateKey::Name, "bar"),
        );
        let and_with_or = AstNode::LogicalOp(
            LogicalOperator::And,
            predicate(PredicateKey::Ext, "rs"),
            Box::new(inner_or),
        );
        let final_ast = AstNode::LogicalOp(
            LogicalOperator::And,
            Box::new(and_with_or),
            Box::new(AstNode::Not(predicate(PredicateKey::Path, "tests"))),
        );
        assert_eq!(ast, final_ast);
    }

    #[test]
    fn test_unescape_value() {
        assert_eq!(unescape_value(r#""hello "world"""#), "hello \"world\"");
        assert_eq!(unescape_value(r#"'hello 'world''"#), "hello 'world'");
        assert_eq!(unescape_value("no_quotes"), "no_quotes");
    }

    #[test]
    fn test_parse_predicate_with_special_chars_in_value() {
        let ast = parse_query(r#"name:"foo&bar""#).unwrap();
        assert_eq!(ast, *predicate(PredicateKey::Name, "foo&bar"));
    }

    #[test]
    fn test_parse_semantic_predicates() {
        let ast_def = parse_query("def:User").unwrap();
        assert_eq!(ast_def, *predicate(PredicateKey::Def, "User"));

        let ast_func = parse_query("func:get_user").unwrap();
        assert_eq!(ast_func, *predicate(PredicateKey::Func, "get_user"));

        let ast_import = parse_query("import:serde").unwrap();
        assert_eq!(ast_import, *predicate(PredicateKey::Import, "serde"));
    }
}
