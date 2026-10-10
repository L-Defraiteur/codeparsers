//! **Ce qu'on ne sait pas relier est dit, jamais jeté.**
//!
//! Le relevé des références non résolues ne comptait que les imports, et
//! seulement entre fichiers : en « fichier seul » (le mode de rag3weaver), il
//! rendait 0 sur le C++ de rag3db, où 89 % des appels relevés ne recevaient
//! aucune relation (10 octobre 2026). Chaque référence d'un scope qui ne
//! devient pas une relation doit y figurer, avec sa raison.

use std::collections::HashMap;

use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};
use codeparsers::relationship_resolution::types::{RelationshipResolutionResult, RelationshipResolverOptions};

fn resoudre(fichiers: &[(&str, &str)], fichier_seul: bool) -> RelationshipResolutionResult {
    let mut contenus = HashMap::new();
    let mut chemins = Vec::new();
    for (nom, source) in fichiers {
        let chemin = format!("/virtual/{nom}");
        contenus.insert(chemin.clone(), source.to_string());
        chemins.push(chemin);
    }
    ProjectParser::new(ProjectParserOptions { verbose: false })
        .parse_project(ParseProjectOptions {
            root: "/virtual".to_string(),
            files: chemins,
            content_map: Some(contenus),
            resolve_relationships: Some(true),
            resolver_options: Some(RelationshipResolverOptions {
                include_file_level_refs: Some(false),
                include_child_refs: Some(false),
                resolve_cross_file: Some(!fichier_seul),
                ..Default::default()
            }),
        })
        .relationships
        .expect("relations")
}

const PROJET: &[(&str, &str)] = &[
    (
        "a.rs",
        "pub fn helper() -> u32 {\n    1\n}\n\npub fn run() -> u32 {\n    helper() + absente() + ailleurs() + double()\n}\n\nmod m1 {\n    pub fn double() -> u32 {\n        1\n    }\n}\n\nmod m2 {\n    pub fn double() -> u32 {\n        2\n    }\n}\n",
    ),
    ("b.rs", "pub fn ailleurs() -> u32 {\n    3\n}\n"),
];

fn raison<'a>(r: &'a RelationshipResolutionResult, de: &str, nom: &str) -> Option<&'a str> {
    r.unresolved_references.iter().find(|u| u.from_scope == de && u.identifier == nom).map(|u| u.reason.as_str())
}

#[test]
fn en_fichier_seul_ce_qui_n_est_pas_relie_est_releve() {
    let r = resoudre(PROJET, true);
    eprintln!("{:#?}", r.unresolved_references);
    assert!(raison(&r, "run", "helper").is_none(), "helper est relié dans son fichier");
    assert!(raison(&r, "run", "absente").is_some(), "un nom qu'aucun scope ne porte");
    assert!(raison(&r, "run", "ailleurs").is_some(), "défini dans un autre fichier : au consommateur, mais dit");
    assert_eq!(r.stats.unresolved_count as usize, r.unresolved_references.len(), "le compteur dit la liste");
    assert!(r.stats.unresolved_count > 0.0);
}

#[test]
fn la_raison_distingue_les_cas() {
    let r = resoudre(PROJET, true);
    let absente = raison(&r, "run", "absente").unwrap_or_default();
    let ailleurs = raison(&r, "run", "ailleurs").unwrap_or_default();
    assert_ne!(absente, ailleurs, "« aucun scope » n'est pas « dans un autre fichier »");
    // Entre fichiers, `ailleurs` se relie et sort du relevé.
    let croise = resoudre(PROJET, false);
    assert!(raison(&croise, "run", "ailleurs").is_none(), "{:#?}", croise.unresolved_references);
    assert!(raison(&croise, "run", "absente").is_some());
}
