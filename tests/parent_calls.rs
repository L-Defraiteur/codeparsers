//! **The extraction does not walk the tree down from the root for each identifier** (rag3db, 19 November 2026).
//! `Node::parent` of tree-sitter descends from the root at each call; the shared extraction asked it for each
//! identifier, about half of the analysis in a sampled release profile. A file per language of the router, with a
//! few hundred identifiers: the extraction of each walks down the tree at most a handful of times
//! (`parents::tree_parent_calls`), and still reads the call it makes. A count, not a clock.

use std::collections::HashMap;

use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};
use codeparsers::scope_extraction::parents::tree_parent_calls;

const STATEMENTS: usize = 200;

fn body(line: impl Fn(usize) -> String) -> String {
    (0..STATEMENTS).map(line).collect::<Vec<_>>().join("")
}

fn sources() -> Vec<(&'static str, String)> {
    vec![
        ("calls.rs", format!("pub fn f(a: u32) -> u32 {{\n{}    a\n}}\n", body(|i| format!("    let v{i} = g(a, {i});\n")))),
        ("calls.c", format!("int f(int a) {{\n{}    return a;\n}}\n", body(|i| format!("    int v{i} = g(a, {i});\n")))),
        ("calls.cpp", format!("int f(int a) {{\n{}    return a;\n}}\n", body(|i| format!("    int v{i} = g(a, {i});\n")))),
        ("calls.py", format!("def f(a):\n{}    return a\n", body(|i| format!("    v{i} = g(a, {i})\n")))),
        ("calls.ts", format!("export function f(a: number): number {{\n{}  return a;\n}}\n", body(|i| format!("  const v{i} = g(a, {i});\n")))),
        ("calls.go", format!("package m\nfunc f(a int) int {{\n{}    return a\n}}\n", body(|i| format!("    v{i} := g(a, {i}); _ = v{i}\n")))),
        ("calls.cs", format!("class C {{ int f(int a) {{\n{}    return a;\n}} }}\n", body(|i| format!("    var v{i} = g(a, {i});\n")))),
    ]
}

#[test]
fn the_extraction_does_not_walk_down_the_tree_for_each_identifier() {
    let mut walked = Vec::new();
    for (name, source) in sources() {
        let path = format!("/virtual/{name}");
        let before = tree_parent_calls();
        let a = ProjectParser::new(ProjectParserOptions { verbose: false }).parse_project(ParseProjectOptions {
            root: "/virtual".to_string(),
            files: vec![path.clone()],
            content_map: Some(HashMap::from([(path, source)])),
            resolve_relationships: Some(false),
            resolver_options: None,
        });
        let calls = tree_parent_calls() - before;
        let calls_g = a.files.values().flat_map(|f| f.scopes.iter().filter(|s| s.name == "f")).any(|s| s.identifier_references.iter().any(|r| r.identifier == "g"));
        assert!(calls_g, "{name}: the function still reads its call to g");
        println!("{name}: {calls} walks down the tree");
        if calls > 10 {
            walked.push(format!("{name} {calls}"));
        }
    }
    assert!(walked.is_empty(), "{STATEMENTS} calls to g, the tree walked down from the root for each: {walked:?}");
}
