//! **The Python and C stop words are Python's and C's** (rag3db ticket « la liste des mots vides de Rust est
//! une copie de celle de JavaScript », 18 November 2026, Python and C): drawn from the language itself
//! (`data/stop_words/python.toml`, `data/stop_words/c.toml`), not the JavaScript list. A Python or C name that
//! is a JavaScript keyword (`default`, `of`, `function`, `this`) is a reference; a keyword of the language is not.

use std::collections::HashMap;

use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};

fn identifiers(file: &str, scope: &str, source: &str) -> Vec<String> {
    let path = format!("/virtual/{file}");
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
fn a_python_name_that_is_a_javascript_keyword_is_a_reference() {
    let src = "def helper():\n    return 2\n\ndef default():\n    return 1\n\ndef of(x):\n    return x\n\ndef this():\n    return 3\n\nclass C:\n    def function(self):\n        return 0\n\ndef start():\n    return helper() + default() + of(2) + C().function() + this()\n";
    let ids = identifiers("lib.py", "start", src);
    assert!(ids.iter().any(|i| i == "helper"), "the control name is read: {ids:?}");
    for name in ["default", "of", "function", "this"] {
        assert!(ids.iter().any(|i| i == name), "{name} is a reference: {ids:?}");
    }
    for name in ["return", "def", "None"] {
        assert!(!ids.iter().any(|i| i == name), "{name} is not: {ids:?}");
    }
}

#[test]
fn a_c_name_that_is_a_javascript_keyword_is_a_reference() {
    let src = "int helper(int x) { return x; }\nint from(int x) { return x; }\nint of(int x) { return x; }\nint function(int x) { return x; }\nint this(void) { return 4; }\nint start(void) { return helper(0) + from(1) + of(2) + function(3) + this(); }\n";
    let ids = identifiers("lib.c", "start", src);
    assert!(ids.iter().any(|i| i == "helper"), "the control name is read: {ids:?}");
    for name in ["from", "of", "function", "this"] {
        assert!(ids.iter().any(|i| i == name), "{name} is a reference: {ids:?}");
    }
    for name in ["return", "int"] {
        assert!(!ids.iter().any(|i| i == name), "{name} is not: {ids:?}");
    }
}
