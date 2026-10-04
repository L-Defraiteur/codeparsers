//! **Un scope ne se consomme pas lui-même.** `struct ExportFuncLocalState {
//! virtual ~ExportFuncLocalState() = default; }` : le nom du destructeur
//! sortait comme une référence de la classe à son propre nom, et la
//! résolution des références locales, qui ne vérifiait pas que la cible n'est
//! pas la source, posait `ExportFuncLocalState → ExportFuncLocalState` (52
//! boucles sur le dépôt rag3db). Et le nom d'un destructeur n'est pas un
//! usage de la classe : c'est un nom déclaré.

use std::collections::HashMap;

use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};
use codeparsers::relationship_resolution::types::RelationshipType;

const H: &str = "namespace function {\n\nstruct ExportFuncLocalState {\n    virtual ~ExportFuncLocalState() = default;\n\n    int cast() {\n        return 1;\n    }\n};\n\n}\n";

#[test]
fn une_classe_ne_se_consomme_pas_par_son_destructeur() {
    let chemin = "/virtual/export_function.h".to_string();
    let a = ProjectParser::new(ProjectParserOptions { verbose: false }).parse_project(ParseProjectOptions {
        root: "/virtual".to_string(),
        files: vec![chemin.clone()],
        content_map: Some(HashMap::from([(chemin.clone(), H.to_string())])),
        resolve_relationships: Some(true),
        resolver_options: None,
    });
    let rels = a.relationships.expect("relations").relationships;
    let boucles: Vec<String> = rels.iter().filter(|r| r.from_uuid == r.to_uuid).map(|r| format!("{:?} {} → {}", r.r#type, r.from_name, r.to_name)).collect();
    assert!(boucles.is_empty(), "un scope ne se consomme pas lui-même : {boucles:?}");
    let consomme_la_classe: Vec<String> = rels
        .iter()
        .filter(|r| r.r#type == RelationshipType::CONSUMES && r.to_name == "ExportFuncLocalState")
        .map(|r| r.from_name.clone())
        .collect();
    assert!(consomme_la_classe.is_empty(), "le nom du destructeur n'est pas un usage de la classe : {consomme_la_classe:?}");
}
