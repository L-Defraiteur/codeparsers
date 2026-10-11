//! **La variable d'une boucle C++ sur intervalle a le type de l'élément**
//! (B2d, pendant C++ du cas T3, 11 octobre 2026). `for (auto& n : v)
//! { n.poids(); }` gardait l'appel sans type : relié au mieux par le seul
//! nom. Un type écrit (`for (Noeud* n : v)`) se lit tel quel ; `auto` prend
//! l'élément de la collection, lu sur la déclaration de `v` dans la
//! fonction (paramètre ou locale).

use std::collections::HashMap;

use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};

fn types_des_appels(source: &str, scope: &str, methode: &str) -> Vec<Option<String>> {
    let chemin = "/virtual/a.cpp".to_string();
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

const SRC: &str = "class Noeud { public: int poids(); };\n\
void a(const std::vector<Noeud>& v) {\n    for (auto& n : v) { n.poids(); }\n}\n\
void b(std::vector<std::unique_ptr<Noeud>>& v) {\n    for (const auto& n : v) { n->poids(); }\n}\n\
void c(std::vector<Noeud*> v) {\n    for (Noeud* n : v) { n->poids(); }\n}\n\
void d() {\n    std::vector<Noeud> v;\n    for (auto n : v) { n.poids(); }\n}\n\
void e(const std::unordered_map<int, Noeud>& m) {\n    for (auto& p : m) { p.poids(); }\n}\n\
void f() {\n    for (auto& n : inconnu) { n.poids(); }\n}\n";

#[test]
fn auto_prend_l_element_de_la_collection_declaree() {
    for f in ["a", "b", "d"] {
        assert_eq!(types_des_appels(SRC, f, "poids"), vec![Some("Noeud".to_string())], "{f}");
    }
}

#[test]
fn un_type_ecrit_se_lit_tel_quel() {
    assert_eq!(types_des_appels(SRC, "c", "poids"), vec![Some("Noeud".to_string())]);
}

#[test]
fn une_paire_ou_une_collection_inconnue_reste_sans_type() {
    for f in ["e", "f"] {
        assert!(types_des_appels(SRC, f, "poids").iter().all(|t| t.is_none()), "{f}");
    }
}
