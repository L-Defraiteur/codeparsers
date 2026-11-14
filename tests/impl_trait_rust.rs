//! **Un `impl Trait for Type` Rust se dit IMPLEMENTS sous toutes ses formes** (14 novembre 2026, point 4
//! d'archi) : l'expression `impl\s+\w+\s+for` manquait un bloc générique (`impl<T> Outil for Pile<T>`), un
//! trait écrit avec son chemin (`impl crate::Outil for Marteau`) ou générique (`impl Conv<u8> for Clou`),
//! et le nom du trait gardait son chemin.

use std::collections::HashMap;

use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};
use codeparsers::relationship_resolution::types::{RelationshipResolverOptions, RelationshipType};

const SRC: &str = "pub trait Outil {\n    fn jouer(&self) -> i32;\n}\n\npub trait Conv<X> {\n    fn de(x: X) -> Self;\n}\n\npub struct Pile<T> {\n    v: Vec<T>,\n}\n\npub struct Marteau;\n\npub struct Clou;\n\nimpl<T> Outil for Pile<T> {\n    fn jouer(&self) -> i32 { 1 }\n}\n\nimpl crate::Outil for Marteau {\n    fn jouer(&self) -> i32 { 2 }\n}\n\nimpl Conv<u8> for Clou {\n    fn de(_x: u8) -> Self { Clou }\n}\n";

#[test]
fn un_impl_generique_ou_a_chemin_implemente_son_trait() {
    let chemin = "/virtual/lib.rs".to_string();
    let r = ProjectParser::new(ProjectParserOptions { verbose: false })
        .parse_project(ParseProjectOptions {
            root: "/virtual".to_string(),
            files: vec![chemin.clone()],
            content_map: Some(HashMap::from([(chemin.clone(), SRC.to_string())])),
            resolve_relationships: Some(true),
            resolver_options: Some(RelationshipResolverOptions {
                include_file_level_refs: Some(false),
                include_child_refs: Some(false),
                resolve_cross_file: Some(false),
                project_files: Some(vec![chemin.clone()]),
                ..Default::default()
            }),
        })
        .relationships
        .expect("une résolution");
    let implements: Vec<(String, String)> = r
        .relationships
        .iter()
        .filter(|x| x.r#type == RelationshipType::IMPLEMENTS)
        .map(|x| (x.from_name.clone(), x.to_name.clone()))
        .collect();
    for (de, vers) in [("Pile<T>", "Outil"), ("Marteau", "Outil"), ("Clou", "Conv")] {
        assert!(
            implements.iter().any(|(d, v)| d.split('<').next() == de.split('<').next() && v == vers),
            "{de} implémente {vers} : {implements:?}"
        );
    }
}
