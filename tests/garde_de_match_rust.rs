//! **Une garde de `match` est du code, pas un motif.** `collect_pattern_bindings` tenait tous les
//! enfants d'un `match_pattern` pour un motif, y compris son champ `condition` (la garde `if …`) : les
//! noms de la garde devenaient des liaisons locales, et l'appel qu'elle porte disparaissait des
//! références. Trouvé par G et everything declarative le 7 novembre 2026 : `vue_portee`, appelée
//! seulement dans `("GET", p) if vue_portee(p) => {` (rag3weaver, src/ide.rs), avait 0 usage.

use std::collections::HashMap;

use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};

const SOURCE: &str = "pub fn route(m: &str, chemin: &str) -> u32 {\n    match (m, chemin) {\n        (\"GET\", p) if vue_portee(p) => 1,\n        (\"POST\", q) if est_court(q) && pret() => 2,\n        _ => 0,\n    }\n}\n\npub fn suite(x: Option<u32>, mut it: std::vec::IntoIter<u32>) -> u32 {\n    let mut n = 0;\n    if let Some(y) = lire(x) {\n        n += y;\n    }\n    while let Some(z) = it.next() {\n        n += z + borne();\n    }\n    n\n}\n\nfn vue_portee(p: &str) -> bool {\n    !p.is_empty()\n}\n\nfn est_court(q: &str) -> bool {\n    q.len() < 4\n}\n\nfn pret() -> bool {\n    true\n}\n\nfn lire(x: Option<u32>) -> Option<u32> {\n    x\n}\n\nfn borne() -> u32 {\n    1\n}\n";

fn references_de(nom: &str) -> Vec<String> {
    let chemin = "/virtual/garde.rs".to_string();
    let a = ProjectParser::new(ProjectParserOptions { verbose: false }).parse_project(ParseProjectOptions {
        root: "/virtual".to_string(),
        files: vec![chemin.clone()],
        content_map: Some(HashMap::from([(chemin, SOURCE.to_string())])),
        resolve_relationships: Some(false),
        resolver_options: None,
    });
    a.files
        .values()
        .flat_map(|f| f.scopes.iter().filter(|s| s.name == nom).flat_map(|s| s.identifier_references.iter().map(|r| r.identifier.clone())))
        .collect()
}

#[test]
fn l_appel_d_une_garde_de_match_est_une_reference() {
    let refs = references_de("route");
    assert!(refs.contains(&"vue_portee".to_string()), "l'appel d'une garde simple : {refs:?}");
    assert!(refs.contains(&"est_court".to_string()) && refs.contains(&"pret".to_string()), "les appels d'une garde avec && : {refs:?}");
    for local in ["p", "q"] {
        assert!(!refs.contains(&local.to_string()), "« {local} », lié par le motif, reste une locale : {refs:?}");
    }
}

#[test]
fn if_let_et_while_let_lisent_leur_valeur_et_lient_leur_motif() {
    let refs = references_de("suite");
    assert!(refs.contains(&"lire".to_string()), "la valeur d'un if let : {refs:?}");
    assert!(refs.contains(&"borne".to_string()), "le corps d'un while let : {refs:?}");
    for local in ["y", "z", "n"] {
        assert!(!refs.contains(&local.to_string()), "« {local} » est une locale : {refs:?}");
    }
}
