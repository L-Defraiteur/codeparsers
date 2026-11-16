//! **A tuple-pattern local and a shadowed local read their type** (rag3db ticket « des receveurs restent
//! sans type », 16 November 2026), in one file: the file's finalisation reads the deferred types.

use std::collections::HashMap;

use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};

const SRC: &str = "pub struct Catalog;\nimpl Catalog {\n    pub fn drain(&mut self) -> u8 { 0 }\n}\npub struct Store;\nfn make_pair() -> (Catalog, Store) { (Catalog, Store) }\npub fn checkpoint() -> u8 {\n    let (mut catalog, _store) = make_pair();\n    catalog.drain()\n}\npub struct GraphTool;\nimpl GraphTool {\n    pub fn from_mermaid(t: &str) -> Result<Self, String> { Ok(GraphTool) }\n    pub fn bind(self) -> Result<Self, String> { Ok(self) }\n    pub fn describe(&self) -> u8 { 0 }\n}\npub fn materialiser(t: &str) -> Result<u8, String> {\n    let outil = GraphTool::from_mermaid(t).map_err(|e| e.to_string())?;\n    let outil = outil.bind()?;\n    Ok(outil.describe())\n}\n";

fn type_of(scope: &str, name: &str) -> Vec<Option<String>> {
    let path = "/virtual/lib.rs".to_string();
    let a = ProjectParser::new(ProjectParserOptions { verbose: false }).parse_project(ParseProjectOptions {
        root: "/virtual".to_string(),
        files: vec![path.clone()],
        content_map: Some(HashMap::from([(path, SRC.to_string())])),
        resolve_relationships: Some(false),
        resolver_options: None,
    });
    a.files
        .values()
        .flat_map(|f| f.scopes.iter().filter(|s| s.name == scope).flat_map(|s| s.identifier_references.iter().filter(|r| r.identifier == name && r.qualifier.is_some()).map(|r| r.qualifier_type.clone())))
        .collect()
}

#[test]
fn a_tuple_pattern_local_reads_its_element_type() {
    assert_eq!(type_of("checkpoint", "drain"), vec![Some("Catalog".to_string())]);
}

#[test]
fn a_shadowed_local_reads_the_binding_visible_where_it_is_used() {
    assert_eq!(type_of("materialiser", "bind"), vec![Some("GraphTool".to_string())]);
    assert_eq!(type_of("materialiser", "describe"), vec![Some("GraphTool".to_string())]);
}
