//! **Un appel sur une locale qui reste non typée est visible, non attribué**
//! (décision de Lucie, 11 octobre 2026). Il était écarté en silence (règle
//! d'origine du crate) : `let t = f(); t.update()` ne laissait rien, ni
//! relation ni non-résolue. Il devient une référence marquée
//! `untyped_receiver`, que le résolveur ne relie jamais et qu'il range dans
//! le relevé des non-résolues sous la raison « receveur non typé ».

use std::collections::HashMap;

use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};
use codeparsers::relationship_resolution::types::RelationshipResolverOptions;

/// (nom, qualificatif, marque) des appels `update` du scope `run`, et les
/// raisons des non-résolues `update`, en fichier seul.
fn lire(fichier: &str, source: &str) -> (Vec<(String, Option<String>, bool)>, Vec<String>, usize) {
    let chemin = format!("/virtual/{fichier}");
    let a = ProjectParser::new(ProjectParserOptions { verbose: false }).parse_project(ParseProjectOptions {
        root: "/virtual".to_string(),
        files: vec![chemin.clone()],
        content_map: Some(HashMap::from([(chemin, source.to_string())])),
        resolve_relationships: Some(true),
        resolver_options: Some(RelationshipResolverOptions { resolve_cross_file: Some(false), ..Default::default() }),
    });
    let refs = a
        .files
        .values()
        .flat_map(|fa| fa.scopes.iter())
        .filter(|s| s.name == "run")
        .flat_map(|s| s.identifier_references.iter())
        .filter(|r| r.identifier == "update")
        .map(|r| (r.identifier.clone(), r.qualifier.clone(), r.untyped_receiver))
        .collect();
    let rel = a.relationships.expect("le relevé");
    let raisons = rel.unresolved_references.iter().filter(|u| u.identifier == "update" && u.from_scope == "run").map(|u| u.reason.clone()).collect();
    let aretes = rel.relationships.iter().filter(|x| x.from_name == "run" && x.to_name == "update").count();
    (refs, raisons, aretes)
}

/// Une méthode `update` dans le fichier : sans la marque, le résolveur
/// pourrait relier `t.update()` à elle par le seul nom.
#[test]
fn rust_let_d_un_appel_inconnu() {
    let src = "pub struct Table;\nimpl Table {\n    pub fn update(&self) {}\n}\npub fn run() {\n    let t = f();\n    t.update();\n}\n";
    let (refs, raisons, aretes) = lire("lib.rs", src);
    assert_eq!(refs, vec![("update".to_string(), Some("t".to_string()), true)], "visible et marqué");
    assert_eq!(raisons, vec!["receveur non typé".to_string()]);
    assert_eq!(aretes, 0, "jamais une arête");
}

#[test]
fn cpp_auto_d_un_appel_inconnu() {
    let src = "class Table { public: void update(); };\nvoid Table::update() {}\nvoid run() {\n    auto t = f();\n    t.update();\n}\n";
    let (refs, raisons, aretes) = lire("a.cpp", src);
    assert_eq!(refs, vec![("update".to_string(), Some("t".to_string()), true)], "visible et marqué");
    assert_eq!(raisons, vec!["receveur non typé".to_string()]);
    assert_eq!(aretes, 0, "jamais une arête");
}

/// Une locale typée garde son type et sa résolution : la marque ne touche
/// que ce qui ne se lit pas.
#[test]
fn une_locale_typee_n_est_pas_marquee() {
    let src = "pub struct Table;\nimpl Table {\n    pub fn update(&self) {}\n}\npub fn run() {\n    let t = Table;\n    t.update();\n}\n";
    let (refs, raisons, aretes) = lire("lib.rs", src);
    assert_eq!(refs, vec![("update".to_string(), Some("t".to_string()), false)]);
    assert!(raisons.is_empty(), "{raisons:?}");
    assert_eq!(aretes, 1);
}

/// Le cas écarté en silence : la variable d'une boucle sur une collection
/// inconnue. Avant, `o.update()` ne laissait aucune référence.
#[test]
fn un_appel_ecarte_en_silence_devient_visible() {
    let src = "pub struct Table;\nimpl Table {\n    pub fn update(&self) {}\n}\npub fn run() {\n    for o in fabrique() {\n        o.update();\n    }\n}\n";
    let (refs, raisons, aretes) = lire("lib.rs", src);
    assert_eq!(refs, vec![("update".to_string(), Some("o".to_string()), true)], "visible et marqué");
    assert_eq!(raisons, vec!["receveur non typé".to_string()]);
    assert_eq!(aretes, 0);
}
