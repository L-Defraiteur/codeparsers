//! **Un bloc `impl` se dit tel** (T1 de la session recherche, 11 octobre
//! 2026) : il a le nom de son type et le genre `Class`, comme la struct ;
//! sans marque, un consommateur y voyait une seconde définition de
//! `Marteau`, et le nom devenait ambigu partout où un `impl` existe. Le
//! modificateur `impl` le distingue ; la struct ne le porte pas.

use std::collections::HashMap;

use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};

#[test]
fn un_bloc_impl_porte_le_modificateur_impl() {
    let src = "pub trait Outil { fn jouer(&self) -> i32; }\npub struct Marteau;\nimpl Outil for Marteau {\n    fn jouer(&self) -> i32 { 1 }\n}\nimpl Marteau {\n    pub fn neuf() -> Self { Marteau }\n}\n";
    let chemin = "/virtual/traits.rs".to_string();
    let a = ProjectParser::new(ProjectParserOptions { verbose: false }).parse_project(ParseProjectOptions {
        root: "/virtual".to_string(),
        files: vec![chemin.clone()],
        content_map: Some(HashMap::from([(chemin, src.to_string())])),
        resolve_relationships: Some(false),
        resolver_options: None,
    });
    let marteaux: Vec<(String, bool)> = a
        .files
        .values()
        .flat_map(|fa| fa.scopes.iter())
        .filter(|s| s.name == "Marteau")
        .map(|s| (s.signature.clone(), s.modifiers.iter().any(|m| m == "impl")))
        .collect();
    assert_eq!(
        marteaux,
        vec![("pub struct Marteau".to_string(), false), ("impl Outil for Marteau".to_string(), true), ("impl Marteau".to_string(), true)]
    );
}

/// **Une méthode d'un `impl Trait for Type` porte `trait_impl`** ; la méthode propre du type, non.
/// À nom et type égaux, un consommateur fait passer la méthode propre avant (19 octobre 2026).
#[test]
fn une_methode_d_impl_de_trait_porte_trait_impl() {
    let src = "pub trait Lire { fn get(&self) -> i32; }\npub struct Catalogue;\nimpl Lire for Catalogue {\n    fn get(&self) -> i32 { 1 }\n}\nimpl Catalogue {\n    pub fn get(&self) -> i32 { 2 }\n}\n";
    let chemin = "/virtual/lire.rs".to_string();
    let a = ProjectParser::new(ProjectParserOptions { verbose: false }).parse_project(ParseProjectOptions {
        root: "/virtual".to_string(),
        files: vec![chemin.clone()],
        content_map: Some(HashMap::from([(chemin, src.to_string())])),
        resolve_relationships: Some(false),
        resolver_options: None,
    });
    let mut gets: Vec<(Option<String>, bool)> = a
        .files
        .values()
        .flat_map(|fa| fa.scopes.iter())
        .filter(|s| s.name == "get" && s.parent.as_deref() == Some("Catalogue"))
        .map(|s| (s.parent.clone(), s.modifiers.iter().any(|m| m == "trait_impl")))
        .collect();
    gets.sort();
    assert_eq!(gets, vec![(Some("Catalogue".to_string()), false), (Some("Catalogue".to_string()), true)]);
}
