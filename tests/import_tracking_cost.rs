//! **Tracking the imports of a scope does not compile a regular expression per import** (rag3db, 19 November 2026:
//! `ensure_import_references_tracked` built `\bname\b` for each scope and each import not yet tracked — 23 % of the
//! analysis of rag3weaver/src in a sampled profile). A file of 300 imports and 300 functions, one of which uses one
//! import: the analysis stays under a second, and finds that use, and only it. The imports are external (`serde::`):
//! only those go through the tracking (`structured_imports`), a local `crate::` import does not.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};

fn source() -> String {
    let mut s = String::new();
    for i in 0..300 {
        s.push_str(&format!("use serde::Name{i};\n"));
    }
    for f in 0..300 {
        s.push_str(&format!("pub fn f{f}(x: u32) -> u32 {{\n"));
        for l in 0..10 {
            s.push_str(&format!("    let y{l} = x + {l}; // Named{l} is not a whole word of an import\n"));
        }
        if f == 42 {
            s.push_str("    let n = Name7::new();\n");
        }
        s.push_str("    x\n}\n");
    }
    s
}

#[test]
fn tracking_the_imports_of_many_scopes_stays_cheap_and_exact() {
    let path = "/virtual/lib.rs".to_string();
    let t = Instant::now();
    let a = ProjectParser::new(ProjectParserOptions { verbose: false }).parse_project(ParseProjectOptions {
        root: "/virtual".to_string(),
        files: vec![path.clone()],
        content_map: Some(HashMap::from([(path, source())])),
        resolve_relationships: Some(false),
        resolver_options: None,
    });
    let took = t.elapsed();
    let uses = |scope: &str| -> Vec<String> {
        a.files.values().flat_map(|f| f.scopes.iter().filter(|s| s.name == scope).flat_map(|s| s.identifier_references.iter().flat_map(|r| std::iter::once(r.identifier.clone()).chain(r.qualifier.clone())))).filter(|i| i.starts_with("Name")).collect()
    };
    assert!(uses("f42").iter().any(|i| i == "Name7"), "the import used is a reference (as a name or a qualifier): {:?}", uses("f42"));
    assert!(uses("f41").is_empty(), "an import not used is not: {:?}", uses("f41"));
    assert!(took < Duration::from_secs(1), "300 imports by 300 scopes in {took:?}");
}
