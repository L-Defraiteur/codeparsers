//! **Un nom dans les arguments d'une macro Rust se lit comme hors macro.**
//!
//! tree-sitter-rust ne lit pas les arguments d'une macro comme des
//! expressions : `assert_eq!(crate::b::run(), 2)` n'y est qu'une suite de
//! jetons (`crate`, `::`, `b`, `::`, `run`, `(`…). Sans relecture, `run`
//! sortait nu, sans qualificatif ni origine d'import, et le consommateur ne
//! pouvait le relier que par le seul nom. Chaque expression ci-dessous doit
//! rendre, dans une macro, les mêmes références qu'hors macro : nom,
//! qualificatif, type du qualificatif, origine d'import.

use std::collections::{BTreeSet, HashMap};

use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};
use codeparsers::scope_extraction::types::IdentifierReference;

fn refs(source: &str) -> Vec<(String, IdentifierReference)> {
    let chemin = "/virtual/lib.rs".to_string();
    let a = ProjectParser::new(ProjectParserOptions { verbose: false }).parse_project(ParseProjectOptions {
        root: "/virtual".to_string(),
        files: vec![chemin.clone()],
        content_map: Some(HashMap::from([(chemin, source.to_string())])),
        resolve_relationships: Some(false),
        resolver_options: None,
    });
    a.files.values().flat_map(|fa| fa.scopes.iter().flat_map(|s| s.identifier_references.iter().map(|r| (s.name.clone(), r.clone())))).collect()
}

/// Les références d'un scope, sans celle du nom de la macro, dans une forme
/// comparable : (nom, qualificatif, type du qualificatif, origine).
fn lues(r: &[(String, IdentifierReference)], scope: &str) -> BTreeSet<(String, Option<String>, Option<String>, Option<String>)> {
    r.iter()
        .filter(|(s, x)| s == scope && x.identifier != "assert")
        .map(|(_, x)| {
            let origine = x.import_origin.as_ref().map(|o| format!("{}|{}|{}", o.source, o.imported, o.via_qualifier));
            (x.identifier.clone(), x.qualifier.clone(), x.qualifier_type.clone(), origine)
        })
        .collect()
}

const ENTETE: &str = "use crate::connection;\n\npub struct Outil;\n\nimpl Outil {\n    pub fn fabrique() -> bool {\n        true\n    }\n    pub fn pret(&self) -> bool {\n        true\n    }\n}\n\n";

#[test]
fn une_expression_se_lit_pareil_dans_une_macro() {
    let expressions = [
        "crate::b::run()",
        "connection::open()",
        "Outil::fabrique()",
        "o.pret()",
        "self.a.pret()",
    ];
    for e in expressions {
        let src = format!(
            "{ENTETE}pub struct S {{\n    a: Outil,\n}}\n\nimpl S {{\n    pub fn hors(&self, o: Outil) -> bool {{\n        {e}\n    }}\n    pub fn dans(&self, o: Outil) {{\n        assert!({e});\n    }}\n}}\n"
        );
        let r = refs(&src);
        let (hors, dans) = (lues(&r, "hors"), lues(&r, "dans"));
        assert!(!hors.is_empty(), "{e} : rien hors macro");
        assert_eq!(dans, hors, "{e} : la macro ne se lit pas comme hors macro");
    }
}

#[test]
fn le_cas_du_ticket_garde_son_chemin() {
    // Ticket « le qualificatif d'un chemin se perd dans une macro ».
    let src = "#[cfg(test)]\nmod tests {\n    #[test]\n    fn dans_macro() {\n        assert_eq!(crate::b::run(), 2);\n    }\n}\n";
    let r = refs(src);
    let run: Vec<&IdentifierReference> = r.iter().filter(|(s, x)| s == "dans_macro" && x.identifier == "run").map(|(_, x)| x).collect();
    assert_eq!(run.len(), 1, "{r:#?}");
    assert_eq!(run[0].qualifier.as_deref(), Some("crate::b"));
    // Comme hors macro : la tête `crate` ne sort pas, le segment `b` porte
    // le qualificatif de ce qui le précède.
    let b: Vec<Option<&str>> = r.iter().filter(|(s, x)| s == "dans_macro" && x.identifier == "b").map(|(_, x)| x.qualifier.as_deref()).collect();
    assert_eq!(b, vec![Some("crate")], "{r:#?}");
}

#[test]
fn un_attribut_de_compilation_n_est_pas_une_reference() {
    let src = "#[cfg(test)]\nmod tests {\n    #[test]\n    #[allow(dead_code)]\n    fn t() {\n        run();\n    }\n}\n";
    let r = refs(src);
    let noms: BTreeSet<&str> = r.iter().map(|(_, x)| x.identifier.as_str()).collect();
    assert!(!noms.contains("cfg") && !noms.contains("test") && !noms.contains("allow") && !noms.contains("dead_code"), "{noms:?}");
    assert!(noms.contains("run"), "le corps reste lu : {noms:?}");
}

#[test]
fn un_type_a_chemin_garde_son_chemin() {
    let src = "pub fn f(d: std::sync::Arc<dyn crate::dialect::SchemaDialect>) -> crate::x::Rate {\n    todo!()\n}\n";
    let r = refs(src);
    let q = |nom: &str| r.iter().find(|(_, x)| x.identifier == nom).and_then(|(_, x)| x.qualifier.clone());
    assert_eq!(q("SchemaDialect").as_deref(), Some("crate::dialect"));
    assert_eq!(q("Rate").as_deref(), Some("crate::x"));
}

#[test]
fn un_receveur_se_lit_a_travers_un_verrou() {
    let src = "use std::sync::{Arc, Mutex};\n\npub fn f(catalog: Arc<Mutex<Catalog>>, v: Vec<u8>) {\n    let cat = catalog.lock().unwrap();\n    cat.ingest_entities();\n    catalog.lock().unwrap().vider();\n    let n: usize = v.iter().map(|x| *x as usize).count();\n}\n";
    let r = refs(src);
    let t = |nom: &str| r.iter().find(|(_, x)| x.identifier == nom).unwrap_or_else(|| panic!("{nom} absent : {r:#?}")).1.qualifier_type.clone();
    assert_eq!(t("ingest_entities").as_deref(), Some("Catalog"), "par une variable liée à la chaîne");
    assert_eq!(t("vider").as_deref(), Some("Catalog"), "sur la chaîne elle-même");
    assert_eq!(t("count").as_deref(), Some("Iterator"), "une méthode d'itérateur vise un type std");
}

#[test]
fn un_retour_ou_un_champ_se_pele_par_sa_chaine() {
    // Les deux arêtes qui bloquaient le filtre d'arête dans `impact` :
    // `setup().lock().unwrap().ingest_entities()` (retour d'une fonction) et
    // `guard.probe()` avec `guard = self.catalog.lock().unwrap()` (un champ).
    let src = "use std::sync::{Arc, Mutex};\n\npub struct Catalog;\n\npub struct Svc {\n    catalog: Arc<Mutex<Catalog>>,\n}\n\nfn setup() -> Arc<Mutex<Catalog>> {\n    todo!()\n}\n\nimpl Svc {\n    fn f(&self) {\n        let guard = self.catalog.lock().unwrap();\n        guard.probe();\n    }\n}\n\nfn t() {\n    let catalog = setup();\n    catalog.lock().unwrap().ingest_entities();\n    setup().lock().expect(\"verrou\").vider();\n}\n";
    let r = refs(src);
    let t = |nom: &str| r.iter().find(|(_, x)| x.identifier == nom).unwrap_or_else(|| panic!("{nom} absent : {r:#?}")).1.qualifier_type.clone();
    assert_eq!(t("ingest_entities").as_deref(), Some("Catalog"), "variable liée au retour de setup, pelée");
    assert_eq!(t("vider").as_deref(), Some("Catalog"), "retour de setup, pelé directement");
    assert_eq!(t("probe").as_deref(), Some("Catalog"), "variable liée au champ catalog, pelée");
}

#[test]
fn une_lecture_de_champ_n_est_pas_une_reference() {
    let src = "pub fn f(r: Fenetre) -> bool {\n    let a = r.indexed_hash.is_none();\n    assert!(r.outside && r.indexed_hash.is_none());\n    r.indexed_hash()\n}\n";
    let r = refs(src);
    let lignes: Vec<usize> = r.iter().filter(|(_, x)| x.identifier == "indexed_hash").map(|(_, x)| x.line).collect();
    assert_eq!(lignes, vec![4], "seul l'appel de méthode reste : {r:#?}");
    assert!(!r.iter().any(|(_, x)| x.identifier == "outside"), "{r:#?}");
}

#[test]
fn un_constructeur_enveloppant_se_pele() {
    let src = "use std::sync::{Arc, Mutex};\n\npub struct Catalog;\n\nfn catalogue(n: u32) -> Catalog {\n    todo!()\n}\n\nfn t() {\n    let catalog = Arc::new(Mutex::new(catalogue(4)));\n    catalog.lock().unwrap().ingest_entities();\n}\n";
    let r = refs(src);
    let t = r.iter().find(|(_, x)| x.identifier == "ingest_entities").map(|(_, x)| x.qualifier_type.clone());
    assert_eq!(t, Some(Some("Catalog".to_string())), "{r:#?}");
}

#[test]
fn un_motif_deballe_son_element_la_ou_il_vaut() {
    // `estimate_of` (rag3weaver) : le paramètre `catalog: Option<&Arc<Mutex<Catalog>>>`,
    // redéclaré par `Some(catalog)` — deux types, chacun à sa place.
    let src = "use std::sync::{Arc, Mutex};\n\nfn estimate_of(catalog: Option<&Arc<Mutex<Catalog>>>) {\n    let x = catalog.is_some();\n    match catalog {\n        None => {}\n        Some(catalog) => {\n            let guard = catalog.lock().unwrap();\n            guard.probe_embedding_rate();\n        }\n    }\n    if let Some(c) = catalog {\n        c.lock().unwrap().vider();\n    }\n}\n";
    let r = refs(src);
    let t = |nom: &str| r.iter().find(|(_, x)| x.identifier == nom).unwrap_or_else(|| panic!("{nom} absent : {r:#?}")).1.qualifier_type.clone();
    assert_eq!(t("probe_embedding_rate").as_deref(), Some("Catalog"));
    assert_eq!(t("vider").as_deref(), Some("Catalog"));
    assert_eq!(t("is_some").as_deref(), Some("Option"), "hors du bras, le paramètre garde son type");
}
