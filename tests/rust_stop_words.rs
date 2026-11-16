//! **The Rust stop words are Rust's** (rag3db ticket « la liste des mots vides de Rust est une copie de
//! celle de JavaScript », 16 November 2026): drawn from the std doc (`data/stop_words/rust.toml`), not the
//! JavaScript list. A Rust name that is a JavaScript keyword (`default`, `from`, `class`) is a reference;
//! a Rust keyword and a prelude variant (`Some`) are not.

use std::collections::HashMap;

use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};

fn identifiers(scope: &str, source: &str) -> Vec<String> {
    let path = "/virtual/lib.rs".to_string();
    let a = ProjectParser::new(ProjectParserOptions { verbose: false }).parse_project(ParseProjectOptions {
        root: "/virtual".to_string(),
        files: vec![path.clone()],
        content_map: Some(HashMap::from([(path, source.to_string())])),
        resolve_relationships: Some(false),
        resolver_options: None,
    });
    a.files.values().flat_map(|f| f.scopes.iter().filter(|s| s.name == scope).flat_map(|s| s.identifier_references.iter().map(|r| r.identifier.clone()))).collect()
}

#[test]
fn a_rust_name_that_is_a_javascript_keyword_is_a_reference() {
    let src = "pub struct Settings;\nimpl Settings {\n    pub fn class(&self) -> u8 { 0 }\n}\npub fn start(x: Option<u8>) -> u8 {\n    let s = Settings::default();\n    let n = u32::from(1u8);\n    match x { Some(v) => v + s.class(), None => 0 }\n}\n";
    let ids = identifiers("start", src);
    for name in ["default", "from", "class"] {
        assert!(ids.iter().any(|i| i == name), "{name} is a reference: {ids:?}");
    }
    for name in ["Some", "None", "let", "match"] {
        assert!(!ids.iter().any(|i| i == name), "{name} is not: {ids:?}");
    }
}
