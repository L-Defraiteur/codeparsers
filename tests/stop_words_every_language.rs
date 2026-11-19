//! **The stop words and the built-in names of each language are its own** (rag3db ticket « les mots vides de C++, C# et
//! Go copient JavaScript », 19 November 2026; Lucie's rule: every language in one lot). C++, C# and Go copied the
//! JavaScript stop words (`function`, `of`, `undefined`…), and C#, Go and Python its built-in names (`Symbol`, `Map`,
//! `Number`…): a function or a class of the project named so was never a reference. One case per language.

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
    a.files
        .values()
        .flat_map(|f| f.scopes.iter().filter(|s| s.name == scope).flat_map(|s| s.identifier_references.iter().flat_map(|r| std::iter::once(r.identifier.clone()).chain(r.qualifier.clone()))))
        .collect()
}

/// `(language, file, scope, source, names that must be references, names that must not)`.
fn cases() -> Vec<(&'static str, &'static str, &'static str, &'static str, Vec<&'static str>, Vec<&'static str>)> {
    vec![
        (
            "C++",
            "lib.cpp",
            "start",
            "struct Symbol { int v; };\nint of(int x) { return x; }\nint function(int x) { return x; }\nint start() { Symbol s; return of(1) + function(2) + s.v; }\n",
            vec!["of", "function", "Symbol"],
            vec!["return", "int"],
        ),
        (
            "C#",
            "Lib.cs",
            "Start",
            "class Symbol { }\nclass C {\n    int of(int x) { return x; }\n    int function(int x) { return x; }\n    int Start() { var s = new Symbol(); return of(1) + function(2); }\n}\n",
            vec!["of", "function", "Symbol"],
            vec!["return", "var"],
        ),
        (
            "Go",
            "lib.go",
            "start",
            "package m\ntype Symbol struct{ v int }\nfunc of(x int) int { return x }\nfunc function(x int) int { return x }\nfunc start() int { s := Symbol{}; return of(1) + function(2) + s.v }\n",
            vec!["of", "function", "Symbol"],
            vec!["return", "func"],
        ),
        ("Python", "lib.py", "start", "class Symbol:\n    pass\n\nclass Map:\n    pass\n\ndef start():\n    return Symbol(), Map()\n", vec!["Symbol", "Map"], vec!["return", "def"]),
    ]
}

#[test]
fn a_name_that_javascript_reserves_is_a_reference_in_every_other_language() {
    let mut missing = Vec::new();
    for (language, file, scope, source, must, must_not) in cases() {
        let ids = identifiers(file, scope, source);
        for name in must {
            if !ids.iter().any(|i| i == name) {
                missing.push(format!("{language}: {name} is not a reference ({ids:?})"));
            }
        }
        for name in must_not {
            assert!(!ids.iter().any(|i| i == name), "{language}: {name} is a keyword, not a reference: {ids:?}");
        }
    }
    assert!(missing.is_empty(), "{missing:#?}");
}
