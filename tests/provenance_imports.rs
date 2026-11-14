//! **La provenance d'un nom importé** (14 novembre 2026, décision de Lucie) : sur le nœud `Library`
//! (une par bibliothèque) et sur l'arête `liaison` (une par mention) — `builtin` (bibliothèque standard,
//! listes fixes par langage), `project` (un module du dépôt), `inconnue` sinon ; `dependency` demande les
//! manifestes et vient du consommateur (rag3weaver). Trois cas par langage, options de rag3weaver.

use codeparsers::import_resolution::provenance::Provenance;
use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};
use codeparsers::relationship_resolution::types::{RelationshipResolutionResult, RelationshipResolverOptions};
use std::collections::HashMap;

fn resoudre(fichiers: &[(&str, &str)]) -> RelationshipResolutionResult {
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
            files: chemins.clone(),
            content_map: Some(contenus),
            resolve_relationships: Some(true),
            resolver_options: Some(RelationshipResolverOptions {
                include_file_level_refs: Some(false),
                include_child_refs: Some(false),
                resolve_cross_file: Some(false),
                project_files: Some(chemins),
                ..Default::default()
            }),
        })
        .relationships
        .expect("une résolution")
}

/// La provenance de la bibliothèque `nom` (le nœud `Library`), `None` si elle n'existe pas.
fn bibliotheque(r: &RelationshipResolutionResult, nom: &str) -> Option<Option<Provenance>> {
    r.external_libraries.values().find(|l| l.name == nom).map(|l| l.provenance)
}

/// Les provenances des arêtes `liaison` (CONSUMES, external_binding) qui partent de `de`, réunies.
fn liaisons(r: &RelationshipResolutionResult, de: &str) -> Vec<Provenance> {
    let mut v: Vec<Provenance> = r
        .relationships
        .iter()
        .filter(|x| format!("{:?}", x.r#type) == "CONSUMES" && x.from_name == de)
        .filter_map(|x| x.metadata.as_ref())
        .filter(|m| m.external_binding == Some(true))
        .flat_map(|m| m.provenances.clone())
        .collect();
    v.sort_by_key(|p| p.as_str());
    v.dedup();
    v
}

#[test]
fn rust_std_une_crate_et_un_module_du_depot() {
    let lib = "mod outils;\nuse std::fs::File;\nuse serde::Serialize;\nuse crate::outils::lire;\n\npub fn ouvrir() -> File {\n    lire();\n    File::open(\"x\").unwrap()\n}\n";
    let r = resoudre(&[("lib.rs", lib), ("outils.rs", "pub fn lire() {}\n")]);
    let noms: Vec<_> = r.external_libraries.values().map(|l| (l.name.clone(), l.provenance)).collect();
    let std = r.external_libraries.values().find(|l| l.name.starts_with("std")).map(|l| l.provenance);
    assert_eq!(std, Some(Some(Provenance::Builtin)), "std:: est la bibliothèque standard : {noms:?}");
    let serde = r.external_libraries.values().find(|l| l.name.starts_with("serde")).map(|l| l.provenance);
    assert_eq!(serde, Some(Some(Provenance::Unknown)), "une crate : inconnue sans le Cargo.lock : {noms:?}");
    assert!(!r.external_libraries.values().any(|l| l.name.contains("outils")), "un module du dépôt n'est pas une bibliothèque : {noms:?}");
}

#[test]
fn node_le_moteur_un_paquet_et_un_fichier_relatif() {
    let source = r#""use strict";

const path = require("node:path");
const fs = require("fs");
const _ = require("lodash");
const absent = require("./absent.js");

function lancer() {
  return path.join(fs.readdirSync("."), _.head([1]), absent.x);
}

module.exports = lancer;
"#;
    let r = resoudre(&[("main.js", source)]);
    let noms: Vec<_> = r.external_libraries.values().map(|l| (l.name.clone(), l.provenance)).collect();
    assert_eq!(bibliotheque(&r, "node:path"), Some(Some(Provenance::Builtin)), "{noms:?}");
    assert_eq!(bibliotheque(&r, "fs"), Some(Some(Provenance::Builtin)), "{noms:?}");
    assert_eq!(bibliotheque(&r, "lodash"), Some(Some(Provenance::Unknown)), "un paquet npm : inconnue sans le package.json : {noms:?}");
    // Une seule arête `liaison` de `lancer` vers la zone des quatre `require` : elle dit les trois
    // provenances distinctes de ses mentions.
    assert_eq!(liaisons(&r, "lancer"), vec![Provenance::Builtin, Provenance::Unknown, Provenance::Project]);
}

#[test]
fn python_os_un_paquet_et_un_module_local() {
    let source = "import os\nimport requests\nimport outils\n\ndef lire():\n    outils.f()\n    return os.getcwd(), requests.get('x')\n";
    let r = resoudre(&[("main.py", source), ("outils.py", "def f():\n    pass\n")]);
    let noms: Vec<_> = r.external_libraries.values().map(|l| (l.name.clone(), l.provenance)).collect();
    assert_eq!(bibliotheque(&r, "os"), Some(Some(Provenance::Builtin)), "{noms:?}");
    assert_eq!(bibliotheque(&r, "requests"), Some(Some(Provenance::Unknown)), "un paquet PyPI : inconnue sans les manifestes : {noms:?}");
    assert_eq!(bibliotheque(&r, "outils"), None, "un module du dépôt n'est pas une bibliothèque : {noms:?}");
}
