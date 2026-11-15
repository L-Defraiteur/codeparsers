//! **Une chaîne de méthodes du projet sur un type connu** (ticket rag3db « des receveurs restent
//! sans type », 15 novembre 2026) : `self.teammate_files()?`, `ctx.cancellation()` avec `ctx: &Ctx`,
//! `GraphTool::from_mermaid(t).map_err(…)?` donnent un `DeferredType::OfType`, que la finalisation du
//! fichier lit par les retours écrits des méthodes de ce type (`Self` y vaut le type).

use std::collections::HashMap;

use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};
use codeparsers::scope_extraction::types::IdentifierReference;

const SRC: &str = "pub struct Files;\nimpl Files {\n    pub fn append(&self) {}\n}\npub struct Host;\nimpl Host {\n    fn teammate_files(&self) -> Result<Files, String> { Ok(Files) }\n    pub fn post(&self) -> Result<(), String> {\n        let files = self.teammate_files()?;\n        files.append();\n        Ok(())\n    }\n}\npub struct Cancel;\nimpl Cancel {\n    pub fn wait_timeout(&self) -> bool { true }\n}\npub struct Ctx;\nimpl Ctx {\n    pub fn cancellation(&self) -> Cancel { Cancel }\n}\npub struct GraphTool;\nimpl GraphTool {\n    pub fn from_mermaid(t: &str) -> Result<Self, String> { Ok(GraphTool) }\n    pub fn bind(self) -> Result<Self, String> { Ok(self) }\n    pub fn description(&self) -> u8 { 0 }\n}\npub fn trois(ctx: &Ctx) {\n    let c = ctx.cancellation();\n    c.wait_timeout();\n}\npub fn quatre(texte: &str) -> Result<(), String> {\n    let outil = GraphTool::from_mermaid(texte).map_err(|e| e.to_string())?;\n    let _lie = outil.bind()?;\n    Ok(())\n}\npub fn cinq() -> u8 {\n    let back = GraphTool::from_mermaid(\"x\").unwrap();\n    back.description()\n}\n";

fn reference(scope: &str, nom: &str) -> IdentifierReference {
    let chemin = "/virtual/lib.rs".to_string();
    let a = ProjectParser::new(ProjectParserOptions { verbose: false }).parse_project(ParseProjectOptions {
        root: "/virtual".to_string(),
        files: vec![chemin.clone()],
        content_map: Some(HashMap::from([(chemin, SRC.to_string())])),
        resolve_relationships: Some(false),
        resolver_options: None,
    });
    let v: Vec<IdentifierReference> = a
        .files
        .values()
        .flat_map(|f| f.scopes.iter().filter(|s| s.name == scope).flat_map(|s| s.identifier_references.iter().filter(|r| r.identifier == nom && r.qualifier.is_some()).cloned()))
        .collect();
    assert_eq!(v.len(), 1, "{scope} → {nom} : {v:?}");
    v.into_iter().next().unwrap()
}

#[test]
fn une_locale_liee_a_une_methode_par_self_lit_son_type() {
    assert_eq!(reference("post", "append").qualifier_type.as_deref(), Some("Files"));
}

#[test]
fn une_locale_liee_a_une_methode_d_un_parametre_lit_son_type() {
    assert_eq!(reference("trois", "wait_timeout").qualifier_type.as_deref(), Some("Cancel"));
}

#[test]
fn une_locale_liee_a_un_appel_par_chemin_lit_son_type() {
    assert_eq!(reference("quatre", "bind").qualifier_type.as_deref(), Some("GraphTool"), "map_err(…)?");
    assert_eq!(reference("cinq", "description").qualifier_type.as_deref(), Some("GraphTool"), "unwrap()");
}
