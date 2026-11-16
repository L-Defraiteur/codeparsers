//! **A member read at the top of a file keeps its owner** (rag3db ticket « un membre externe platform se
//! lie à un homonyme », 16 November 2026). File-scope references are read on the text; `process.platform`
//! came out as the bare name `platform`, which a consumer linked to the project's `const platform`.

use std::collections::HashMap;

use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};
use codeparsers::scope_extraction::types::{IdentifierReference, ScopeInfoType};

fn file_scope_refs(file: &str, source: &str, name: &str) -> Vec<IdentifierReference> {
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
        .flat_map(|f| f.scopes.iter().filter(|s| s.r#type == ScopeInfoType::Module).flat_map(|s| s.identifier_references.iter().filter(|r| r.identifier == name).cloned()))
        .collect()
}

#[test]
fn a_member_at_the_top_of_a_file_keeps_its_owner() {
    let src = "const join = require(\"path\").join;\nif (process.platform === \"linux\") {\n  process.dlopen(m, p);\n}\n";
    let r = file_scope_refs("native.js", src, "platform");
    assert_eq!(r.len(), 1, "{r:?}");
    assert_eq!(r[0].qualifier.as_deref(), Some("process"), "{r:?}");
}

#[test]
fn a_bare_name_at_the_top_of_a_file_has_no_owner() {
    let src = "compute();\nif (ready) {\n  compute();\n}\n";
    let r = file_scope_refs("bare.js", src, "compute");
    assert!(!r.is_empty() && r.iter().all(|x| x.qualifier.is_none()), "{r:?}");
}
