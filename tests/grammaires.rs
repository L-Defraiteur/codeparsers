//! **Chaque grammaire liée se charge dans le tree-sitter lié.**
//!
//! Une grammaire compilée en ABI 15 s'installe sans broncher à côté de
//! tree-sitter 0.24, qui refuse au-delà de 14 — et ne casse qu'à l'exécution.
//! Le 3 octobre 2026, `tree-sitter-c-sharp` 0.23.5 (une version de correctif !)
//! était dans ce cas : codeparsers verrouillait 0.23.1, mais rag3weaver, qui
//! résout la dépendance par son propre `Cargo.lock`, prenait 0.23.5, et tout
//! fichier `.cs` faisait paniquer l'analyse. Ces tests tournent contre le
//! verrou qui les compile : l'épingle `=` du `Cargo.toml` garantit que ce
//! verrou et celui de rag3weaver ne divergent plus.

use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};
use codeparsers::relationship_resolution::types::RelationshipType;
use std::collections::HashMap;
use tree_sitter::{Language, Parser, LANGUAGE_VERSION, MIN_COMPATIBLE_LANGUAGE_VERSION};

#[test]
fn chaque_grammaire_a_une_abi_que_tree_sitter_accepte() {
    let grammaires: Vec<(&str, Language)> = vec![
        ("typescript", tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()),
        ("python", tree_sitter_python::LANGUAGE.into()),
        ("rust", tree_sitter_rust::LANGUAGE.into()),
        ("go", tree_sitter_go::LANGUAGE.into()),
        ("c", tree_sitter_c::LANGUAGE.into()),
        ("cpp", tree_sitter_cpp::LANGUAGE.into()),
        ("c-sharp", tree_sitter_c_sharp::LANGUAGE.into()),
        ("bash", tree_sitter_bash::LANGUAGE.into()),
        ("css", tree_sitter_css::LANGUAGE.into()),
        ("html", tree_sitter_html::LANGUAGE.into()),
        ("scss", tree_sitter_scss::language()),
    ];
    let mut refusees = Vec::new();
    for (nom, langue) in grammaires {
        let abi = langue.version();
        let mut p = Parser::new();
        if p.set_language(&langue).is_err() {
            refusees.push(format!("{nom} (ABI {abi})"));
        }
    }
    assert!(
        refusees.is_empty(),
        "grammaires refusées par tree-sitter (ABI acceptées : {MIN_COMPATIBLE_LANGUAGE_VERSION}..={LANGUAGE_VERSION}) : {refusees:?}"
    );
}

#[test]
fn un_fichier_csharp_s_analyse_sans_paniquer() {
    let chemin = "/virtual/items.cs".to_string();
    let source = "class Index {}\nclass Item : Index {}\n";
    let analyse = ProjectParser::new(ProjectParserOptions { verbose: false }).parse_project(ParseProjectOptions {
        root: "/virtual".to_string(),
        files: vec![chemin.clone()],
        content_map: Some(HashMap::from([(chemin, source.to_string())])),
        resolve_relationships: Some(true),
        resolver_options: None,
    });
    let relations = analyse.relationships.expect("des relations résolues").relationships;
    assert!(
        relations.iter().any(|r| r.r#type == RelationshipType::INHERITSFROM && r.from_name == "Item" && r.to_name == "Index"),
        "class Item : Index → INHERITS_FROM"
    );
}
