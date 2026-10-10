//! **Le receveur d'une boucle Rust a le type de l'élément** (B2d, cas T3 de
//! la session recherche, 11 octobre 2026). `for o in &outils { o.jouer() }`
//! ne laissait aucune référence : `o` est une locale sans type lisible, et
//! un appel sur une telle locale est écarté (on ne relie pas par le seul
//! nom). Typée par l'élément de la collection, la variable rend l'appel à la
//! méthode de ce type.

use std::collections::HashMap;

use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};

/// Le type du qualificatif de chaque appel `methode` du scope `scope`.
fn types_des_appels(source: &str, scope: &str, methode: &str) -> Vec<Option<String>> {
    let chemin = "/virtual/lib.rs".to_string();
    let a = ProjectParser::new(ProjectParserOptions { verbose: false }).parse_project(ParseProjectOptions {
        root: "/virtual".to_string(),
        files: vec![chemin.clone()],
        content_map: Some(HashMap::from([(chemin, source.to_string())])),
        resolve_relationships: Some(false),
        resolver_options: None,
    });
    a.files
        .values()
        .flat_map(|fa| fa.scopes.iter())
        .filter(|s| s.name == scope)
        .flat_map(|s| s.identifier_references.iter())
        .filter(|r| r.identifier == methode)
        .map(|r| r.qualifier_type.clone())
        .collect()
}

const OUTILS: &str = "pub trait Outil { fn jouer(&self) -> i32; }\npub struct Marteau;\nimpl Outil for Marteau {\n    fn jouer(&self) -> i32 { 1 }\n}\n";

#[test]
fn une_boucle_sur_une_reference_de_vec_type_l_element() {
    let src = format!("{OUTILS}pub fn tous(outils: &Vec<Marteau>) -> i32 {{\n    let mut t = 0;\n    for o in outils {{ t += o.jouer(); }}\n    t\n}}\n");
    assert_eq!(types_des_appels(&src, "tous", "jouer"), vec![Some("Marteau".to_string())]);
}

#[test]
fn une_boucle_sur_iter_et_sur_un_emprunt() {
    let src = format!(
        "{OUTILS}pub fn a(v: Vec<Marteau>) {{\n    for o in v.iter() {{ o.jouer(); }}\n}}\npub fn b(v: &[Marteau]) {{\n    for o in v.iter_mut() {{ o.jouer(); }}\n}}\npub fn c(v: Vec<Marteau>) {{\n    for o in &v {{ o.jouer(); }}\n}}\npub fn d(v: std::collections::HashSet<Marteau>) {{\n    for o in v.into_iter() {{ o.jouer(); }}\n}}\n"
    );
    for f in ["a", "b", "c", "d"] {
        assert_eq!(types_des_appels(&src, f, "jouer"), vec![Some("Marteau".to_string())], "{f}");
    }
}

/// Le cas de la session recherche, mot pour mot : la collection vient d'un
/// `vec![…]` de structs unités.
#[test]
fn une_boucle_sur_un_vec_de_structs_unites() {
    let src = format!("{OUTILS}pub fn tous() -> i32 {{\n    let outils = vec![Marteau, Marteau];\n    let mut total = 0;\n    for o in &outils {{ total += o.jouer(); }}\n    total\n}}\n");
    assert_eq!(types_des_appels(&src, "tous", "jouer"), vec![Some("Marteau".to_string())]);
}

/// `let m = Marteau;` : une struct unité nomme son type (T1/T2 de la session
/// recherche, `frappe`).
#[test]
fn une_struct_unite_nomme_son_type() {
    let src = format!("{OUTILS}pub fn frappe() -> i32 {{\n    let m = Marteau;\n    m.jouer()\n}}\n");
    assert_eq!(types_des_appels(&src, "frappe", "jouer"), vec![Some("Marteau".to_string())]);
}

/// Ce qui ne se lit pas reste écarté : une paire de dictionnaire, une
/// collection de type inconnu, une constante en majuscules.
#[test]
fn ce_qui_ne_se_lit_pas_reste_ecarte() {
    let src = format!(
        "{OUTILS}pub fn a(m: HashMap<String, Marteau>) {{\n    for (k, o) in &m {{ o.jouer(); }}\n}}\npub fn b() {{\n    for o in fabrique() {{ o.jouer(); }}\n}}\npub fn c() {{\n    let x = LIMITE;\n    x.jouer();\n}}\n"
    );
    for f in ["a", "b", "c"] {
        assert!(types_des_appels(&src, f, "jouer").iter().all(|t| t.is_none()), "{f}");
    }
}
