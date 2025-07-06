use anyhow::{anyhow, Result};
use pest::iterators::Pair;
use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "rql.pest"]
pub struct RqlParser;

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum PredicateKey {
    Ext,
    Name,
    Path,
    Contains,
    Matches,
    // --- NEW PREDICATES ---
    Size,
    Modified,
    // A key for testing or unknown predicates
    Other(String),
}

impl PredicateKey {
    fn from_str(s: &str) -> Self {
        match s {
            "ext" => Self::Ext,
            "name" => Self::Name,
            "path" => Self::Path,
            "contains" | "c" => Self::Contains,
            "matches" | "m" => Self::Matches,
            // --- NEW PREDICATES ---
            "size" => Self::Size,
            "modified" => Self::Modified,
            // Any other key is captured here.
            other => Self::Other(other.to_string()),
        }
    }
}

#[derive(Debug)]
pub enum AstNode {
    And(Box<AstNode>, Box<AstNode>),
    Or(Box<AstNode>, Box<AstNode>),
    Not(Box<AstNode>),
    Predicate { key: PredicateKey, value: String },
}

pub fn parse_query(query: &str) -> Result<AstNode> {
    // Check for empty or whitespace-only queries BEFORE parsing.
    if query.trim().is_empty() {
        return Err(anyhow!("Empty query"));
    }

    // Map the pest error to a cleaner message if parsing fails.
    let mut pairs = RqlParser::parse(Rule::query, query)
        .map_err(|e| anyhow!("Syntax error in query: {}", e))?;

    let top_level_pair = pairs.next().unwrap(); // Should not fail after the check above
    build_ast_from_pair(top_level_pair)
}

fn build_ast_from_pair(pair: Pair<Rule>) -> Result<AstNode> {
    match pair.as_rule() {
        Rule::query | Rule::expression => build_ast_from_pair(pair.into_inner().next().unwrap()),
        Rule::logical_or => {
            let mut inner = pair.into_inner();
            let mut ast = build_ast_from_pair(inner.next().unwrap())?;
            while inner.next().is_some() {
                let rhs = build_ast_from_pair(inner.next().unwrap())?;
                ast = AstNode::Or(Box::new(ast), Box::new(rhs));
            }
            Ok(ast)
        }
        Rule::logical_and => {
            let mut inner = pair.into_inner();
            let mut ast = build_ast_from_pair(inner.next().unwrap())?;
            while inner.next().is_some() {
                let rhs = build_ast_from_pair(inner.next().unwrap())?;
                ast = AstNode::And(Box::new(ast), Box::new(rhs));
            }
            Ok(ast)
        }
        Rule::factor => {
            let mut inner = pair.into_inner();
            let first_node = inner.next().unwrap();
            if first_node.as_rule() == Rule::NOT {
                let expr = build_ast_from_pair(inner.next().unwrap())?;
                Ok(AstNode::Not(Box::new(expr)))
            } else {
                build_ast_from_pair(first_node)
            }
        }
        Rule::predicate => {
            let mut inner = pair.into_inner();
            let key_str = inner.next().unwrap().as_str();
            let key = PredicateKey::from_str(key_str);

            let value_pair = inner.next().unwrap();
            let inner_value_pair = value_pair.into_inner().next().unwrap();
            let final_value = match inner_value_pair.as_rule() {
                Rule::unquoted_value => inner_value_pair.as_str().to_string(),
                Rule::quoted_value => {
                    let s = inner_value_pair.as_str();
                    s[1..s.len() - 1].to_string()
                }
                _ => unreachable!(),
            };
            Ok(AstNode::Predicate {
                key,
                value: final_value,
            })
        }
        _ => unreachable!(
            "build_ast_from_pair called on unexpected rule: {:?}",
            pair.as_rule()
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    impl PartialEq for AstNode {
        fn eq(&self, other: &Self) -> bool {
            match (self, other) {
                (AstNode::And(l1, r1), AstNode::And(l2, r2)) => l1 == l2 && r1 == r2,
                (AstNode::Or(l1, r1), AstNode::Or(l2, r2)) => l1 == l2 && r1 == r2,
                (AstNode::Not(n1), AstNode::Not(n2)) => n1 == n2,
                (
                    AstNode::Predicate { key: k1, value: v1 },
                    AstNode::Predicate { key: k2, value: v2 },
                ) => k1 == k2 && v1 == v2,
                _ => false,
            }
        }
    }

    fn predicate(key: PredicateKey, value: &str) -> Box<AstNode> {
        Box::new(AstNode::Predicate {
            key,
            value: value.to_string(),
        })
    }

    #[test]
    fn test_parse_simple_predicate() {
        let ast = parse_query("ext:rs").unwrap();
        assert_eq!(ast, *predicate(PredicateKey::Ext, "rs"));
    }

    #[test]
    fn test_predicate_with_quoted_value() {
        let ast = parse_query("contains:'fn main'").unwrap();
        assert_eq!(ast, *predicate(PredicateKey::Contains, "fn main"));
    }

    #[test]
    fn test_predicate_alias() {
        let ast = parse_query("c:\"some value\"").unwrap();
        assert_eq!(ast, *predicate(PredicateKey::Contains, "some value"));
    }

    #[test]
    fn test_unknown_predicate_key() {
        let ast = parse_query("extension:rs").unwrap();
        assert_eq!(
            ast,
            *predicate(PredicateKey::Other("extension".to_string()), "rs")
        );
    }

    #[test]
    fn test_parse_and_operator() {
        let ast = parse_query("ext:rs & contains:'fn'").unwrap();
        let expected = AstNode::And(
            predicate(PredicateKey::Ext, "rs"),
            predicate(PredicateKey::Contains, "fn"),
        );
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_parse_or_operator() {
        let ast = parse_query("ext:rs | ext:toml").unwrap();
        let expected = AstNode::Or(
            predicate(PredicateKey::Ext, "rs"),
            predicate(PredicateKey::Ext, "toml"),
        );
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_parse_not_operator() {
        let ast = parse_query("!ext:md").unwrap();
        let expected = AstNode::Not(predicate(PredicateKey::Ext, "md"));
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_parse_precedence() {
        let ast = parse_query("ext:rs & name:main | ext:toml").unwrap();
        let expected = AstNode::Or(
            Box::new(AstNode::And(
                predicate(PredicateKey::Ext, "rs"),
                predicate(PredicateKey::Name, "main"),
            )),
            predicate(PredicateKey::Ext, "toml"),
        );
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_parse_parentheses() {
        let ast = parse_query("ext:rs & (name:main | ext:toml)").unwrap();
        let expected = AstNode::And(
            predicate(PredicateKey::Ext, "rs"),
            Box::new(AstNode::Or(
                predicate(PredicateKey::Name, "main"),
                predicate(PredicateKey::Ext, "toml"),
            )),
        );
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_parse_complex_nested_query() {
        let ast = parse_query("!(ext:rs | path:tests) & (contains:'foo' | c:'bar')").unwrap();
        let expected = AstNode::And(
            Box::new(AstNode::Not(Box::new(AstNode::Or(
                predicate(PredicateKey::Ext, "rs"),
                predicate(PredicateKey::Path, "tests"),
            )))),
            Box::new(AstNode::Or(
                predicate(PredicateKey::Contains, "foo"),
                predicate(PredicateKey::Contains, "bar"),
            )),
        );
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_whitespace_insensitivity() {
        let ast = parse_query("  ext:rs   &   (  path:src   )  ").unwrap();
        let expected = AstNode::And(
            predicate(PredicateKey::Ext, "rs"),
            predicate(PredicateKey::Path, "src"),
        );
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_no_whitespace() {
        let ast = parse_query("ext:rs&path:src").unwrap();
        let expected = AstNode::And(
            predicate(PredicateKey::Ext, "rs"),
            predicate(PredicateKey::Path, "src"),
        );
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_deeply_nested_precedence() {
        let ast = parse_query("a:1 | b:2 & c:3 | d:4 & e:5").unwrap();
        let expected = AstNode::Or(
            Box::new(AstNode::Or(
                predicate(PredicateKey::Other("a".to_string()), "1"),
                Box::new(AstNode::And(
                    predicate(PredicateKey::Other("b".to_string()), "2"),
                    // THIS IS THE CORRECTED LINE:
                    predicate(PredicateKey::Contains, "3"),
                )),
            )),
            Box::new(AstNode::And(
                predicate(PredicateKey::Other("d".to_string()), "4"),
                predicate(PredicateKey::Other("e".to_string()), "5"),
            )),
        );
        assert_eq!(ast, expected);
    }

    // --- SYNTAX ERROR TESTS ---

    #[test]
    fn test_error_on_trailing_operator() {
        let result = parse_query("ext:rs &");
        assert!(result.is_err());
    }

    #[test]
    fn test_error_on_missing_value() {
        let result = parse_query("ext:");
        assert!(result.is_err());
    }

    #[test]
    fn test_error_on_unclosed_parenthesis() {
        let result = parse_query("(ext:rs | path:src");
        assert!(result.is_err());
    }

    #[test]
    fn test_error_on_empty_query() {
        let result = parse_query("");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Empty query");
    }

    #[test]
    fn test_error_on_whitespace_query() {
        let result = parse_query("   ");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Empty query");
    }

    #[test]
    fn test_not_precedence_with_and() {
        // `!` should have higher precedence than `&`.
        // Should parse as: (!ext:rs) & path:src
        let ast = parse_query("!ext:rs & path:src").unwrap();
        let expected = AstNode::And(
            Box::new(AstNode::Not(predicate(PredicateKey::Ext, "rs"))),
            predicate(PredicateKey::Path, "src"),
        );
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_not_with_parentheses() {
        // `!` should apply to the entire parenthesized group.
        let ast = parse_query("!(ext:rs | path:src)").unwrap();
        let expected = AstNode::Not(Box::new(AstNode::Or(
            predicate(PredicateKey::Ext, "rs"),
            predicate(PredicateKey::Path, "src"),
        )));
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_long_or_chain_is_left_associative() {
        // Should parse as: ((a | b) | c) | d
        let ast = parse_query("ext:a | ext:b | ext:c | ext:d").unwrap();
        let expected = AstNode::Or(
            Box::new(AstNode::Or(
                Box::new(AstNode::Or(
                    predicate(PredicateKey::Ext, "a"),
                    predicate(PredicateKey::Ext, "b"),
                )),
                predicate(PredicateKey::Ext, "c"),
            )),
            predicate(PredicateKey::Ext, "d"),
        );
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_long_and_chain_is_left_associative() {
        // Should parse as: ((a & b) & c) & d
        let ast = parse_query("ext:a & ext:b & ext:c & ext:d").unwrap();
        let expected = AstNode::And(
            Box::new(AstNode::And(
                Box::new(AstNode::And(
                    predicate(PredicateKey::Ext, "a"),
                    predicate(PredicateKey::Ext, "b"),
                )),
                predicate(PredicateKey::Ext, "c"),
            )),
            predicate(PredicateKey::Ext, "d"),
        );
        assert_eq!(ast, expected);
    }

    #[test]
    fn test_redundant_parentheses() {
        let ast = parse_query("((ext:rs))").unwrap();
        assert_eq!(ast, *predicate(PredicateKey::Ext, "rs"));
    }

    #[test]
    fn test_value_containing_special_char_must_be_quoted() {
        // An unquoted value cannot contain '&'
        let result = parse_query("name:foo&bar");
        assert!(result.is_err());

        // But a quoted one can
        let ast = parse_query("name:'foo&bar'").unwrap();
        assert_eq!(ast, *predicate(PredicateKey::Name, "foo&bar"));
    }
}
