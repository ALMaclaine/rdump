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
    Size,
    Modified,
    In,
    // --- SEMANTIC PREDICATES ---
    // Generic
    Def,
    Func,
    Import,
    // Granular Definitions
    Class,
    Struct,
    Enum,
    Interface,
    Trait,
    Type,
    // Syntactic Content
    Comment,
    Str,
    // Usage
    Call,
    // A key for testing or unknown predicates
    Other(String),
}

impl AsRef<str> for PredicateKey {
    fn as_ref(&self) -> &str {
        match self {
            PredicateKey::Ext => "ext",
            PredicateKey::Name => "name",
            PredicateKey::Path => "path",
            PredicateKey::Contains => "contains",
            PredicateKey::Matches => "matches",
            PredicateKey::Size => "size",
            PredicateKey::Modified => "modified",
            PredicateKey::In => "in",
            PredicateKey::Def => "def",
            PredicateKey::Func => "func",
            PredicateKey::Import => "import",
            PredicateKey::Class => "class",
            PredicateKey::Struct => "struct",
            PredicateKey::Enum => "enum",
            PredicateKey::Interface => "interface",
            PredicateKey::Trait => "trait",
            PredicateKey::Type => "type",
            PredicateKey::Comment => "comment",
            PredicateKey::Str => "str",
            PredicateKey::Call => "call",
            PredicateKey::Other(s) => s.as_str(),
        }
    }
}

impl From<&str> for PredicateKey {
    fn from(s: &str) -> Self {
        match s {
            "ext" => Self::Ext,
            "name" => Self::Name,
            "path" => Self::Path,
            "contains" => Self::Contains,
            "matches" => Self::Matches,
            "size" => Self::Size,
            "modified" => Self::Modified,
            "in" => Self::In,
            // --- SEMANTIC ---
            "def" => Self::Def,
            "func" => Self::Func,
            "import" => Self::Import,
            "class" => Self::Class,
            "struct" => Self::Struct,
            "enum" => Self::Enum,
            "interface" => Self::Interface,
            "trait" => Self::Trait,
            "type" => Self::Type,
            "comment" => Self::Comment,
            "str" => Self::Str,
            "call" => Self::Call,
            // Any other key is captured here.
            other => Self::Other(other.to_string()),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum AstNode {
    Predicate(PredicateKey, String),
    LogicalOp(LogicalOperator, Box<AstNode>, Box<AstNode>),
    Not(Box<AstNode>),
}

#[derive(Debug, PartialEq, Clone)]
pub enum LogicalOperator {
    And,
    Or,
}

pub fn parse_query(query: &str) -> Result<AstNode> {
    // Check for empty or whitespace-only queries BEFORE parsing.
    if query.trim().is_empty() {
        return Err(anyhow!("Query cannot be empty."));
    }

    match RqlParser::parse(Rule::query, query) {
        Ok(pairs) => build_ast_from_pairs(pairs.peek().unwrap()),
        Err(e) => {
            // Re-format the pest error to be more user-friendly.
            Err(anyhow!("Invalid query syntax:\n{}", e))
        }
    }
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
        let op = match op_pair.as_str().to_lowercase().as_str() {
            "&" | "and" => LogicalOperator::And,
            "|" | "or" => LogicalOperator::Or,
            _ => unreachable!(),
        };
        let right_pair = inner_pairs.next().unwrap();
        let right_ast = build_ast_from_pairs(right_pair)?;
        ast = AstNode::LogicalOp(op, Box::new(ast), Box::new(right_ast));
    }
    Ok(ast)
}

fn unescape_value(value: &str) -> String {
    let quote_char = value.chars().next();
    if quote_char == Some('"') || quote_char == Some('\'') {
        let inner = &value[1..value.len() - 1];
        let mut unescaped = String::with_capacity(inner.len());
        let mut chars = inner.chars();
        while let Some(c) = chars.next() {
            if c == '\\' {
                if let Some(next_c) = chars.next() {
                    unescaped.push(next_c);
                }
            } else {
                unescaped.push(c);
            }
        }
        return unescaped;
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
        assert_eq!(unescape_value(r#""hello \"world\"""#), "hello \"world\"");
        assert_eq!(unescape_value(r#"'hello \'world\''"#), "hello 'world'");
        assert_eq!(unescape_value(r#""a \\ b""#), "a \\ b");
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

    #[test]
    fn test_parse_granular_and_syntactic_predicates() {
        assert_eq!(
            parse_query("class:Foo").unwrap(),
            *predicate(PredicateKey::Class, "Foo")
        );
        assert_eq!(
            parse_query("struct:Bar").unwrap(),
            *predicate(PredicateKey::Struct, "Bar")
        );
        assert_eq!(
            parse_query("comment:TODO").unwrap(),
            *predicate(PredicateKey::Comment, "TODO")
        );
        assert_eq!(
            parse_query("str:'api_key'").unwrap(),
            *predicate(PredicateKey::Str, "api_key")
        );
        assert_eq!(
            parse_query("call:my_func").unwrap(),
            *predicate(PredicateKey::Call, "my_func")
        );
    }

    #[test]
    fn test_error_on_trailing_operator() {
        let result = parse_query("ext:rs &");
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Invalid query syntax:"));
        assert!(err.to_string().contains("expected")); // Pest's pointer is still useful
    }

    #[test]
    fn test_error_on_missing_value() {
        let result = parse_query("ext:");
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Invalid query syntax:"));
    }

    #[test]
    fn test_error_on_unclosed_parenthesis() {
        let result = parse_query("(ext:rs | path:src");
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Invalid query syntax:"));
    }

    #[test]
    fn test_error_on_empty_query() {
        let result = parse_query("");
        assert_eq!(result.unwrap_err().to_string(), "Query cannot be empty.");
    }

    #[test]
    fn test_error_on_whitespace_query() {
        let result = parse_query("   ");
        assert_eq!(result.unwrap_err().to_string(), "Query cannot be empty.");
    }

    #[test]
    fn test_parse_keyword_operators() {
        // AND
        let ast_and = parse_query("ext:rs and name:\"foo\"").unwrap();
        assert_eq!(
            ast_and,
            AstNode::LogicalOp(
                LogicalOperator::And,
                predicate(PredicateKey::Ext, "rs"),
                predicate(PredicateKey::Name, "foo")
            )
        );

        // OR
        let ast_or = parse_query("ext:rs or ext:toml").unwrap();
        assert_eq!(
            ast_or,
            AstNode::LogicalOp(
                LogicalOperator::Or,
                predicate(PredicateKey::Ext, "rs"),
                predicate(PredicateKey::Ext, "toml")
            )
        );

        // NOT
        let ast_not = parse_query("not ext:rs").unwrap();
        assert_eq!(ast_not, AstNode::Not(predicate(PredicateKey::Ext, "rs")));
    }

    #[test]
    fn test_parse_mixed_operators() {
        let ast = parse_query("ext:rs and (name:foo or name:bar) & not path:tests").unwrap();
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
    fn test_parse_unknown_predicate() {
        let ast = parse_query("unknown:predicate").unwrap();
        assert_eq!(
            ast,
            *predicate(PredicateKey::Other("unknown".to_string()), "predicate")
        );
    }
}
