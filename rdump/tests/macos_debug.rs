// tests/macos_debug.rs

extern crate rdump;

use rdump::parser::{Parser, RqlParser, Rule};


#[test]
fn test_macos_problematic_query() {
    let query = "in:**/shared/*/use-boolean and ext:ts";
    let result = RqlParser::parse(Rule::query, query);
    assert!(result.is_ok(), "Parser failed to parse the query '{:?}'. Error: {:?}", query, result.err().unwrap());
}
