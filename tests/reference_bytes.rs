//! **Each reference says where it is written, to the byte** (lot 2 of the intra-scope: the occurrences in the graph).
//!
//! A tool that rewrites code, or `usages` that gives a column, needs the exact place of each name in the file. The
//! parser reads it on the identifier's node: `start_byte`..`end_byte`, so that `content[start_byte..end_byte]` is the
//! name as written. A reference that does not come from a node (a type read in the text of a signature) has no
//! range: nothing is invented.

use std::collections::HashMap;

use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};
use codeparsers::scope_extraction::types::ScopeInfo;

fn scopes(chemin: &str, source: &str) -> Vec<ScopeInfo> {
    let chemin = format!("/virtual/{chemin}");
    let a = ProjectParser::new(ProjectParserOptions { verbose: false }).parse_project(ParseProjectOptions {
        root: "/virtual".to_string(),
        files: vec![chemin.clone()],
        content_map: Some(HashMap::from([(chemin, source.to_string())])),
        resolve_relationships: Some(false),
        resolver_options: None,
    });
    a.files.into_values().flat_map(|fa| fa.scopes).collect()
}

/// In each language, `caller` calls `helper` twice on two lines: each call is a reference with its own range.
const CAS: &[(&str, &str)] = &[
    ("lib.rs", "fn helper() -> u32 {\n    1\n}\n\nfn caller() -> u32 {\n    helper()\n        + helper()\n}\n"),
    ("mod.py", "def helper():\n    return 1\n\n\ndef caller():\n    a = helper()\n    return a + helper()\n"),
    ("mod.ts", "function helper(): number {\n  return 1;\n}\n\nfunction caller(): number {\n  const a = helper();\n  return a + helper();\n}\n"),
    ("mod.go", "package m\n\nfunc helper() int {\n\treturn 1\n}\n\nfunc caller() int {\n\ta := helper()\n\treturn a + helper()\n}\n"),
    ("mod.c", "int helper(void) {\n    return 1;\n}\n\nint caller(void) {\n    int a = helper();\n    return a + helper();\n}\n"),
    ("mod.cpp", "int helper() {\n    return 1;\n}\n\nint caller() {\n    int a = helper();\n    return a + helper();\n}\n"),
    ("Mod.cs", "class M {\n    static int Helper() {\n        return 1;\n    }\n\n    static int Caller() {\n        int a = Helper();\n        return a + Helper();\n    }\n}\n"),
];

#[test]
fn each_call_of_a_name_has_its_own_byte_range_in_every_language() {
    let mut fautes = Vec::new();
    for (chemin, source) in CAS {
        let s = scopes(chemin, source);
        let appelant = s.iter().find(|x| x.name.eq_ignore_ascii_case("caller")).unwrap_or_else(|| panic!("{chemin}: no caller scope"));
        let nom = if chemin.ends_with(".cs") { "Helper" } else { "helper" };
        let attendus: Vec<usize> = source.match_indices(&format!("{nom}()")).map(|(i, _)| i).filter(|i| *i > source.find("caller").or(source.find("Caller")).unwrap()).collect();
        let lus: Vec<(Option<usize>, Option<usize>)> =
            appelant.identifier_references.iter().filter(|r| r.identifier == nom).map(|r| (r.start_byte, r.end_byte)).collect();
        let mut debuts: Vec<usize> = lus.iter().filter_map(|(d, _)| *d).collect();
        debuts.sort();
        if debuts != attendus {
            fautes.push(format!("{chemin}: the calls of {nom} start at bytes {attendus:?}, the references say {lus:?}"));
        }
        for (d, f) in lus.iter().filter_map(|(d, f)| Some(((*d)?, (*f)?))) {
            if source.get(d..f) != Some(nom) {
                fautes.push(format!("{chemin}: the range {d}..{f} reads {:?}, not {nom}", source.get(d..f)));
            }
        }
    }
    assert!(fautes.is_empty(), "{}", fautes.join("\n"));
}

/// Whatever the language, a range a reference gives is the identifier as written: never a guess.
#[test]
fn a_byte_range_always_reads_the_identifier() {
    for (chemin, source) in CAS {
        for s in scopes(chemin, source) {
            for r in &s.identifier_references {
                if let (Some(d), Some(f)) = (r.start_byte, r.end_byte) {
                    assert_eq!(source.get(d..f), Some(r.identifier.as_str()), "{chemin}: {} in {}", r.identifier, s.name);
                }
            }
        }
    }
}
