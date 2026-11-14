//! **La provenance par les manifestes du dépôt** (14 novembre 2026, second lot de la provenance) : le
//! consommateur lit Cargo.lock, package.json, pyproject.toml, requirements.txt, vcpkg.json et les
//! workspaces, et passe à codeparsers les dépendances déclarées (`declared_dependencies`) et les paquets
//! du dépôt (`project_packages`) ; un import de l'un d'eux est `dependency` ou `project`, au lieu de
//! `inconnue`. Sans ces options, rien ne change (le témoin de contrôle).

use codeparsers::import_resolution::provenance::Provenance;
use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};
use codeparsers::relationship_resolution::types::{RelationshipResolutionResult, RelationshipResolverOptions};
use std::collections::HashMap;

fn resoudre(fichiers: &[(&str, &str)], manifestes: Option<(&[&str], &[&str])>) -> RelationshipResolutionResult {
    let mut contenus = HashMap::new();
    let mut chemins = Vec::new();
    for (nom, source) in fichiers {
        let chemin = format!("/virtual/{nom}");
        contenus.insert(chemin.clone(), source.to_string());
        chemins.push(chemin);
    }
    let (deps, projet) = manifestes.map_or((None, None), |(d, p)| (Some(d.iter().map(|s| s.to_string()).collect()), Some(p.iter().map(|s| s.to_string()).collect())));
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
                declared_dependencies: deps,
                project_packages: projet,
                ..Default::default()
            }),
        })
        .relationships
        .expect("une résolution")
}

fn bibliotheque(r: &RelationshipResolutionResult, nom: &str) -> Option<Option<Provenance>> {
    r.external_libraries.values().find(|l| l.name == nom).map(|l| l.provenance)
}

const RUST: &str = "use serde_json::Value;\nuse ma_crate::outils;\nuse std::fs;\n\npub fn lire() -> Value {\n    outils::f();\n    fs::read(\"x\");\n    Value::Null\n}\n";
const NODE: &str = "\"use strict\";\n\nconst _ = require(\"lodash/fp\");\nconst o = require(\"@moi/outils\");\nconst p = require(\"node:path\");\nconst x = require(\"inconnu-du-manifeste\");\n\nfunction lancer() {\n  return [_.head, o.f, p.sep, x.y];\n}\n\nmodule.exports = lancer;\n";
const PYTHON: &str = "import os\nimport requests\nimport yaml\n\ndef lire():\n    return os.getcwd(), requests.get, yaml.load\n";

#[test]
fn sans_manifestes_tout_reste_inconnu() {
    let r = resoudre(&[("lib.rs", RUST)], None);
    assert_eq!(bibliotheque(&r, "serde_json"), Some(Some(Provenance::Unknown)));
    assert_eq!(bibliotheque(&r, "ma_crate"), Some(Some(Provenance::Unknown)));
}

#[test]
fn rust_une_crate_du_cargo_lock_et_une_crate_du_workspace() {
    // Le Cargo.lock nomme `serde-json` (avec un tiret) ; le workspace a la crate `ma-crate`.
    let r = resoudre(&[("lib.rs", RUST)], Some((&["serde-json"], &["ma-crate"])));
    assert_eq!(bibliotheque(&r, "serde_json"), Some(Some(Provenance::Dependency)), "une crate du Cargo.lock");
    assert_eq!(bibliotheque(&r, "ma_crate"), Some(Some(Provenance::Project)), "une crate du workspace, importée par son nom");
    assert_eq!(bibliotheque(&r, "std"), Some(Some(Provenance::Builtin)), "std reste la bibliothèque standard");
}

#[test]
fn node_un_paquet_du_package_json_et_un_paquet_du_workspace() {
    let r = resoudre(&[("main.js", NODE)], Some((&["lodash"], &["@moi/outils"])));
    assert_eq!(bibliotheque(&r, "lodash/fp"), Some(Some(Provenance::Dependency)), "lodash/fp est le paquet lodash");
    assert_eq!(bibliotheque(&r, "@moi/outils"), Some(Some(Provenance::Project)), "un paquet du workspace npm");
    assert_eq!(bibliotheque(&r, "node:path"), Some(Some(Provenance::Builtin)));
    assert_eq!(bibliotheque(&r, "inconnu-du-manifeste"), Some(Some(Provenance::Unknown)), "jamais une devinette");
    let liaisons: Vec<Provenance> = r
        .relationships
        .iter()
        .filter(|x| format!("{:?}", x.r#type) == "CONSUMES" && x.from_name == "lancer")
        .filter_map(|x| x.metadata.as_ref())
        .filter(|m| m.external_binding == Some(true))
        .flat_map(|m| m.provenances.clone())
        .collect();
    for p in [Provenance::Builtin, Provenance::Dependency, Provenance::Project, Provenance::Unknown] {
        assert!(liaisons.contains(&p), "la liaison réunit {p:?} : {liaisons:?}");
    }
}

#[test]
fn python_un_paquet_de_pyproject_et_un_nom_a_casse_differente() {
    // pyproject.toml : `requests` ; requirements.txt : `PyYAML` (le module s'appelle `yaml` : pas le même
    // nom, donc inconnue — jamais une devinette sur la correspondance paquet → module).
    let r = resoudre(&[("main.py", PYTHON)], Some((&["requests", "PyYAML"], &[])));
    let noms: Vec<_> = r.external_libraries.values().map(|l| (l.name.clone(), l.provenance)).collect();
    let req = r.external_libraries.values().find(|l| l.name.starts_with("requests")).map(|l| l.provenance);
    assert_eq!(req, Some(Some(Provenance::Dependency)), "import requests est le paquet requests : {noms:?}");
    assert_eq!(bibliotheque(&r, "os"), Some(Some(Provenance::Builtin)));
    assert_eq!(bibliotheque(&r, "yaml"), Some(Some(Provenance::Unknown)), "PyYAML fournit yaml, mais le nom ne le dit pas");
}
