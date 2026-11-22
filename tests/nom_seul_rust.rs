//! **Le nom seul et les segments de module** (ticket du 4 octobre, P1 de la
//! stèle du produit).
//! 1. Un paramètre n'est pas un usage : `fn fmt(&self, f: &mut Formatter)`
//!    laissait `f` dans les références du bloc `impl` (lues sur tout son
//!    corps), relié par le seul nom à une fonction `f` d'un autre fichier.
//! 2. Un segment de chemin qui nomme un module (`dataflow` dans
//!    `crate::dataflow::runtime::lancer()`) n'est pas une référence : vrai,
//!    mais il n'apprend rien et encombre les voisinages. Un segment qui nomme
//!    un type (`DataflowEvent` dans `DataflowEvent::neuf()`) en reste une.

use std::collections::HashMap;

use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};
use codeparsers::scope_extraction::types::ScopeInfo;

fn scopes(source: &str) -> Vec<ScopeInfo> {
    let chemin = "/virtual/src/b.rs".to_string();
    let a = ProjectParser::new(ProjectParserOptions { verbose: false }).parse_project(ParseProjectOptions {
        root: "/virtual".to_string(),
        files: vec![chemin.clone()],
        content_map: Some(HashMap::from([(chemin, source.to_string())])),
        resolve_relationships: Some(false),
        resolver_options: None,
    });
    a.files.into_values().flat_map(|fa| fa.scopes).collect()
}

const SRC: &str = "pub struct B;\nimpl std::fmt::Display for B {\n    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {\n        Ok(())\n    }\n}\npub fn g() {\n    let e = crate::dataflow::DataflowEvent::neuf();\n    crate::dataflow::runtime::lancer();\n}\n";

#[test]
fn un_parametre_n_est_pas_une_reference_du_bloc_impl() {
    let s = scopes(SRC);
    let refs: Vec<String> = s.iter().filter(|x| x.name == "B").flat_map(|x| x.identifier_references.iter().map(|r| r.identifier.clone())).collect();
    assert!(!refs.contains(&"f".to_string()), "le paramètre f n'est pas une référence : {refs:?}");
    // Son type en reste une, **de la méthode qui l'écrit** (22 novembre 2026) : une référence appartient au scope qui
    // la contient, et le bloc `impl` ne garde plus celles du corps de ses méthodes ; il garde son en-tête.
    let fmt: Vec<String> = s.iter().filter(|x| x.name == "fmt").flat_map(|x| x.identifier_references.iter().map(|r| r.identifier.clone())).collect();
    assert!(!fmt.contains(&"f".to_string()), "ni de la méthode : {fmt:?}");
    assert!(fmt.contains(&"Formatter".to_string()), "son type en reste une, de la méthode : {fmt:?}");
    assert!(!refs.contains(&"Formatter".to_string()) && refs.contains(&"Display".to_string()), "le bloc garde son en-tête, pas le corps de sa méthode : {refs:?}");
}

#[test]
fn un_segment_de_module_n_est_pas_une_reference() {
    let s = scopes(SRC);
    let g: Vec<String> = s.iter().filter(|x| x.name == "g").flat_map(|x| x.identifier_references.iter().map(|r| r.identifier.clone())).collect();
    assert!(!g.contains(&"dataflow".to_string()) && !g.contains(&"runtime".to_string()), "pas de segment de module : {g:?}");
    for garde in ["DataflowEvent", "neuf", "lancer"] {
        assert!(g.contains(&garde.to_string()), "{garde} reste une référence : {g:?}");
    }
}
