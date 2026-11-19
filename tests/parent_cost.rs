//! **The parent of a node is read once per tree, not walked down from the root at each identifier** (rag3db,
//! 19 November 2026: `Node::parent` of tree-sitter descends from the root and scans the children of every level; the
//! shared extraction called it for each identifier — about half of the analysis of rag3weaver/src and of the engine
//! in a sampled release profile). One function of 3 000 statements per language: each analysis stays under a bound,
//! and still reads the call it makes.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};

const STATEMENTS: usize = 3000;

fn body(line: impl Fn(usize) -> String) -> String {
    (0..STATEMENTS).map(line).collect::<Vec<_>>().join("")
}

/// `(file name, source)` of one long function, per language.
fn sources() -> Vec<(&'static str, String)> {
    vec![
        ("long.rs", format!("pub fn f(a: u32) -> u32 {{\n{}    a\n}}\n", body(|i| format!("    let v{i} = g(a, {i});\n")))),
        ("long.cpp", format!("int f(int a) {{\n{}    return a;\n}}\n", body(|i| format!("    int v{i} = g(a, {i});\n")))),
        ("long.py", format!("def f(a):\n{}    return a\n", body(|i| format!("    v{i} = g(a, {i})\n")))),
        ("long.ts", format!("export function f(a: number): number {{\n{}  return a;\n}}\n", body(|i| format!("  const v{i} = g(a, {i});\n")))),
    ]
}

#[test]
fn a_long_function_is_analysed_in_linear_time_in_every_language() {
    let mut slow = Vec::new();
    for (name, source) in sources() {
        let path = format!("/virtual/{name}");
        let t = Instant::now();
        let a = ProjectParser::new(ProjectParserOptions { verbose: false }).parse_project(ParseProjectOptions {
            root: "/virtual".to_string(),
            files: vec![path.clone()],
            content_map: Some(HashMap::from([(path, source)])),
            resolve_relationships: Some(false),
            resolver_options: None,
        });
        let took = t.elapsed();
        let calls_g = a.files.values().flat_map(|f| f.scopes.iter().filter(|s| s.name == "f")).any(|s| s.identifier_references.iter().any(|r| r.identifier == "g"));
        assert!(calls_g, "{name}: the function still reads its call to g");
        println!("{name}: {took:?}");
        if took > Duration::from_secs(1) {
            slow.push(format!("{name} {took:?}"));
        }
    }
    assert!(slow.is_empty(), "{STATEMENTS} statements in one function, more than a second: {slow:?}");
}
