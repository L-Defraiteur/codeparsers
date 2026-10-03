//! **Les imports de Rust, Python et C++ sont lus, et les bibliothèques
//! externes deviennent des relations `USES_LIBRARY`.**
//!
//! L'état des lieux du 3 octobre 2026 : Rust, Go, C, C++ et C# passaient par
//! l'extracteur d'imports de TypeScript et n'en relevaient aucun ; Python
//! marquait tout import comme local, donc aucun `USES_LIBRARY`.

use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};
use codeparsers::relationship_resolution::types::{RelationshipResolverOptions, RelationshipType};
use codeparsers::scope_extraction::types::ImportReference;
use std::collections::HashMap;

struct Analyse {
    imports: HashMap<String, Vec<ImportReference>>,
    /// (scope, bibliothèque, symbole)
    bibliotheques: Vec<(String, String, Option<String>)>,
}

fn analyser(fichiers: &[(&str, &str)]) -> Analyse {
    let mut contenus = HashMap::new();
    let mut chemins = Vec::new();
    for (nom, source) in fichiers {
        let chemin = format!("/virtual/{nom}");
        contenus.insert(chemin.clone(), source.to_string());
        chemins.push(chemin);
    }
    let a = ProjectParser::new(ProjectParserOptions { verbose: false }).parse_project(ParseProjectOptions {
        root: "/virtual".to_string(),
        files: chemins,
        content_map: Some(contenus),
        resolve_relationships: Some(true),
        resolver_options: Some(RelationshipResolverOptions {
            include_file_level_refs: Some(false),
            include_child_refs: Some(false),
            ..Default::default()
        }),
    });
    let imports = a.files.iter().map(|(f, fa)| (f.rsplit('/').next().unwrap().to_string(), fa.import_references.clone())).collect();
    let bibliotheques = a
        .relationships
        .map(|r| {
            r.relationships
                .into_iter()
                .filter(|r| r.r#type == RelationshipType::USESLIBRARY)
                .map(|r| (r.from_name, r.to_name, r.metadata.and_then(|m| m.symbol)))
                .collect()
        })
        .unwrap_or_default();
    Analyse { imports, bibliotheques }
}

fn import<'a>(a: &'a Analyse, fichier: &str, nom: &str) -> &'a ImportReference {
    a.imports[fichier]
        .iter()
        .find(|i| i.alias.as_deref().unwrap_or(&i.imported) == nom)
        .unwrap_or_else(|| panic!("pas d'import {nom} dans {fichier} : {:#?}", a.imports[fichier]))
}

// ---------------------------------------------------------------------------
// Rust
// ---------------------------------------------------------------------------

const RUST: &str = r#"use std::sync::Arc;
use std::collections::{HashMap, HashSet as Ensemble};
use serde::Serialize;
use crate::outils::Aide;
use super::voisin;

pub fn partage(x: u32) -> Arc<u32> {
    let mut m: HashMap<u32, u32> = HashMap::new();
    m.insert(x, x);
    Arc::new(x)
}

pub fn local() -> Aide {
    Aide::new()
}
"#;

#[test]
fn rust_chaque_use_est_un_import_avec_sa_ligne() {
    let a = analyser(&[("lib.rs", RUST)]);
    let arc = import(&a, "lib.rs", "Arc");
    assert_eq!((arc.source.as_str(), arc.is_local, arc.line), ("std", false, Some(1)));
    let ens = import(&a, "lib.rs", "Ensemble");
    assert_eq!((ens.source.as_str(), ens.line), ("std", Some(2)));
    assert_eq!(import(&a, "lib.rs", "HashMap").source, "std");
    assert_eq!(import(&a, "lib.rs", "Serialize").source, "serde");
    let aide = import(&a, "lib.rs", "Aide");
    assert!(aide.is_local, "crate:: est local");
    assert!(import(&a, "lib.rs", "voisin").is_local, "super:: est local");
}

#[test]
fn rust_une_bibliotheque_externe_utilisee_fait_un_uses_library() {
    let a = analyser(&[("lib.rs", RUST)]);
    assert!(a.bibliotheques.iter().any(|(s, b, _)| s == "partage" && b == "std"), "{:?}", a.bibliotheques);
    assert!(!a.bibliotheques.iter().any(|(_, b, _)| b == "crate" || b == "super"), "un import local n'est pas une bibliothèque : {:?}", a.bibliotheques);
}

#[test]
fn rust_un_import_local_relie_encore_sa_cible() {
    let outils = "pub struct Aide;\nimpl Aide {\n    pub fn new() -> Self { Aide }\n}\n";
    let rels = {
        let mut contenus = HashMap::new();
        contenus.insert("/virtual/lib.rs".to_string(), RUST.to_string());
        contenus.insert("/virtual/outils.rs".to_string(), outils.to_string());
        ProjectParser::new(ProjectParserOptions { verbose: false })
            .parse_project(ParseProjectOptions {
                root: "/virtual".to_string(),
                files: vec!["/virtual/lib.rs".into(), "/virtual/outils.rs".into()],
                content_map: Some(contenus),
                resolve_relationships: Some(true),
                resolver_options: None,
            })
            .relationships
            .unwrap()
            .relationships
    };
    assert!(
        rels.iter().any(|r| r.r#type == RelationshipType::CONSUMES && r.from_name == "local" && r.to_name == "Aide"),
        "local → Aide, comme avant"
    );
}

// ---------------------------------------------------------------------------
// Python
// ---------------------------------------------------------------------------

const PY: &str = "import os\nimport numpy as np\nfrom .voisin import aide\nfrom paquet.module import Chose\n\ndef travail():\n    os.getcwd()\n    np.zeros(3)\n    aide()\n    Chose()\n";

#[test]
fn python_un_import_absolu_est_externe_sauf_module_du_projet() {
    let a = analyser(&[("app.py", PY), ("paquet/module.py", "class Chose:\n    pass\n")]);
    assert!(!import(&a, "app.py", "os").is_local);
    assert!(!import(&a, "app.py", "np").is_local);
    assert!(import(&a, "app.py", "aide").is_local, "un import relatif est local");
    let libs: Vec<&str> = a.bibliotheques.iter().filter(|(s, _, _)| s == "travail").map(|(_, b, _)| b.as_str()).collect();
    assert!(libs.contains(&"os") && libs.contains(&"numpy"), "{:?}", a.bibliotheques);
    assert!(!libs.contains(&"paquet.module"), "un module du projet n'est pas une bibliothèque : {:?}", a.bibliotheques);
    assert_eq!(import(&a, "app.py", "os").line, Some(1));
}

// ---------------------------------------------------------------------------
// C++
// ---------------------------------------------------------------------------

#[test]
fn cpp_un_include_systeme_est_une_bibliotheque_un_include_local_non() {
    let src = "#include <vector>\n#include <gtest/gtest.h>\n#include \"calc.h\"\n\nint f() { return 0; }\n";
    let a = analyser(&[("calc_test.cpp", src)]);
    let par_source = |s: &str| a.imports["calc_test.cpp"].iter().find(|i| i.source == s).unwrap_or_else(|| panic!("pas d'include {s} : {:#?}", a.imports["calc_test.cpp"])).clone();
    let v = par_source("vector");
    assert_eq!((v.is_local, v.line), (false, Some(1)));
    assert!(!par_source("gtest/gtest.h").is_local);
    assert!(par_source("calc.h").is_local);
    let libs: Vec<&str> = a.bibliotheques.iter().map(|(_, b, _)| b.as_str()).collect();
    assert!(libs.contains(&"vector") && libs.contains(&"gtest/gtest.h"), "{:?}", a.bibliotheques);
    assert!(!libs.contains(&"calc.h"), "{:?}", a.bibliotheques);
}

#[test]
fn rust_un_nom_importe_dans_un_commentaire_ou_une_chaine_n_est_pas_un_usage() {
    let src = "use std::sync::Arc;\n\n/// Rend un Arc, un jour.\npub fn plus_tard() -> u32 {\n    // pas encore d'Arc ici\n    let s = \"Arc\";\n    s.len() as u32\n}\n\npub fn maintenant(x: u32) -> u32 {\n    *Arc::new(x)\n}\n";
    let a = analyser(&[("lib.rs", src)]);
    assert!(a.bibliotheques.iter().any(|(s, b, _)| s == "maintenant" && b == "std"), "{:?}", a.bibliotheques);
    assert!(!a.bibliotheques.iter().any(|(s, _, _)| s == "plus_tard"), "un commentaire ou une chaîne n'utilise rien : {:?}", a.bibliotheques);
}

#[test]
fn rust_un_import_externe_ne_se_resout_pas_vers_un_homonyme_du_projet() {
    // graph_tool.rs : `use std::fmt;` puis `fmt::Formatter` se reliaient à la
    // méthode `fmt` d'un `impl Display` du projet.
    let erreur = "use std::fmt;\n\npub struct Erreur;\n\nimpl fmt::Display for Erreur {\n    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {\n        write!(f, \"erreur\")\n    }\n}\n";
    let autre = "use std::fmt;\n\npub struct Autre;\n\nimpl fmt::Display for Autre {\n    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {\n        write!(f, \"autre\")\n    }\n}\n";
    let mut contenus = HashMap::new();
    contenus.insert("/virtual/erreur.rs".to_string(), erreur.to_string());
    contenus.insert("/virtual/autre.rs".to_string(), autre.to_string());
    let rels = ProjectParser::new(ProjectParserOptions { verbose: false })
        .parse_project(ParseProjectOptions {
            root: "/virtual".to_string(),
            files: vec!["/virtual/erreur.rs".into(), "/virtual/autre.rs".into()],
            content_map: Some(contenus),
            resolve_relationships: Some(true),
            resolver_options: None,
        })
        .relationships
        .unwrap()
        .relationships;
    let fausses: Vec<String> = rels
        .iter()
        .filter(|r| r.r#type == RelationshipType::CONSUMES && r.to_name == "fmt" && r.from_file != r.to_file)
        .map(|r| format!("{} ({}) → fmt ({})", r.from_name, r.from_file, r.to_file))
        .collect();
    assert!(fausses.is_empty(), "std::fmt n'est pas la méthode fmt d'un autre fichier : {fausses:#?}");
}
