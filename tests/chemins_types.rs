//! **Un appel par chemin sur un type a ce type pour receveur** (B2d) :
//! `Catalog::Get(ctx)`, `Rag3dbConnection::in_memory()`. Le qualificatif
//! nomme le type : il devient `qualifier_type`, et le consommateur relie
//! l'appel à la méthode de ce type — ou à rien si le type n'est pas du
//! projet (`Duration::from_millis`), au lieu d'un homonyme par le seul nom.

use std::collections::HashMap;

use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};

/// (qualificatif, type du qualificatif) de chaque appel `methode`.
fn appels(fichier: &str, source: &str, methode: &str) -> Vec<(Option<String>, Option<String>)> {
    let chemin = format!("/virtual/{fichier}");
    let a = ProjectParser::new(ProjectParserOptions { verbose: false }).parse_project(ParseProjectOptions {
        root: "/virtual".to_string(),
        files: vec![chemin.clone()],
        content_map: Some(HashMap::from([(chemin, source.to_string())])),
        resolve_relationships: Some(false),
        resolver_options: None,
    });
    let mut v: Vec<_> = a
        .files
        .values()
        .flat_map(|fa| fa.scopes.iter())
        .flat_map(|s| s.identifier_references.iter())
        .filter(|r| r.identifier == methode)
        .map(|r| (r.qualifier.clone(), r.qualifier_type.clone()))
        .collect();
    v.sort();
    v.dedup();
    v
}

fn t(s: &str) -> Option<String> {
    Some(s.to_string())
}

#[test]
fn rust_un_chemin_qui_finit_par_un_type() {
    let src = "pub fn a() {\n    let c = Rag3dbConnection::in_memory();\n    let o = crate::outils::Outil::fabrique();\n    let d = std::time::Duration::from_millis(3);\n}\n";
    assert_eq!(appels("lib.rs", src, "in_memory"), vec![(t("Rag3dbConnection"), t("Rag3dbConnection"))]);
    assert_eq!(appels("lib.rs", src, "fabrique"), vec![(t("crate::outils::Outil"), t("Outil"))]);
    assert_eq!(appels("lib.rs", src, "from_millis"), vec![(t("std::time::Duration"), t("Duration"))]);
}

#[test]
fn rust_un_chemin_de_module_n_est_pas_un_type() {
    let src = "pub fn a() {\n    crate::outils::run();\n    outils::run2();\n    LIMITE::x();\n}\n";
    assert!(appels("lib.rs", src, "run").iter().all(|(_, ty)| ty.is_none()));
    assert!(appels("lib.rs", src, "run2").iter().all(|(_, ty)| ty.is_none()));
    assert!(appels("lib.rs", src, "x").iter().all(|(_, ty)| ty.is_none()), "une constante en majuscules n'est pas un type");
}

#[test]
fn cpp_un_chemin_qui_finit_par_un_type() {
    let src = "void f(ClientContext* ctx) {\n    auto v = Catalog::Get(*ctx)->getVersion();\n    auto m = storage::StorageManager::Get(*ctx);\n    auto p = IntegerBitpacking<T>::getPackingInfo(x);\n    auto q = std::make_pair(1, 2);\n    auto r = common::stringFormat(\"x\");\n}\n";
    assert_eq!(appels("a.cpp", src, "Get").iter().filter_map(|(_, ty)| ty.clone()).collect::<Vec<_>>(), vec!["Catalog".to_string(), "StorageManager".to_string()]);
    assert!(appels("a.cpp", src, "getPackingInfo").iter().any(|(_, ty)| ty.as_deref() == Some("IntegerBitpacking")));
    assert!(appels("a.cpp", src, "make_pair").iter().all(|(_, ty)| ty.is_none()));
    assert!(appels("a.cpp", src, "stringFormat").iter().all(|(_, ty)| ty.is_none()));
}
