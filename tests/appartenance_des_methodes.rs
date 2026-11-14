//! **L'appartenance d'une méthode hors du bloc de son type** (14 novembre 2026, la navigation members
//! d'archi) : elle lit `parent` (→ `parent_name` à l'index). Un récepteur Go générique (`*Pile[K]`,
//! `Pile[K]`) nomme son type sans les crochets ; une méthode Python a sa classe pour parent, une classe
//! imbriquée la sienne.

use std::collections::HashMap;

use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};

fn scopes(fichier: &str, src: &str) -> Vec<(String, Option<String>, Option<String>)> {
    let chemin = format!("/virtual/{fichier}");
    let a = ProjectParser::new(ProjectParserOptions { verbose: false }).parse_project(ParseProjectOptions {
        root: "/virtual".to_string(),
        files: vec![chemin.clone()],
        content_map: Some(HashMap::from([(chemin, src.to_string())])),
        resolve_relationships: Some(false),
        resolver_options: None,
    });
    let mut v: Vec<(String, Option<String>, Option<String>)> = a.files.values().flat_map(|fa| fa.scopes.iter()).map(|s| (s.name.clone(), s.parent.clone(), s.parent_path.clone())).collect();
    v.sort();
    v
}

fn parents(fichier: &str, src: &str) -> Vec<(String, Option<String>)> {
    let chemin = format!("/virtual/{fichier}");
    let a = ProjectParser::new(ProjectParserOptions { verbose: false }).parse_project(ParseProjectOptions {
        root: "/virtual".to_string(),
        files: vec![chemin.clone()],
        content_map: Some(HashMap::from([(chemin, src.to_string())])),
        resolve_relationships: Some(false),
        resolver_options: None,
    });
    let mut v: Vec<(String, Option<String>)> = a.files.values().flat_map(|fa| fa.scopes.iter()).map(|s| (s.name.clone(), s.parent.clone())).collect();
    v.sort();
    v
}

#[test]
fn go_un_recepteur_generique_nomme_son_type() {
    let src = "package pile\n\ntype Pile[K any] struct {\n\telements []K\n}\n\nfunc (p *Pile[K]) Push(k K) {\n\tp.elements = append(p.elements, k)\n}\n\nfunc (p Pile[K]) Len() int {\n\treturn len(p.elements)\n}\n\nfunc (Pile[K]) Vide() bool {\n\treturn false\n}\n\nfunc (s *Simple) Nom() string {\n\treturn \"\"\n}\n\ntype Simple struct{}\n";
    let p = parents("pile.go", src);
    for m in ["Push", "Len", "Vide"] {
        assert!(p.contains(&(m.to_string(), Some("Pile".to_string()))), "{m} appartient à Pile : {p:?}");
    }
    assert!(p.contains(&("Nom".to_string(), Some("Simple".to_string()))), "{p:?}");
}

#[test]
fn python_une_methode_a_sa_classe_pour_parent() {
    let src = "class Lecteur:\n    def lire(self):\n        return 1\n\n    class Tampon:\n        def vider(self):\n            return 0\n\n\ndef seule():\n    return 2\n";
    let p = parents("lecteur.py", src);
    assert!(p.contains(&("lire".to_string(), Some("Lecteur".to_string()))), "{p:?}");
    assert!(p.contains(&("vider".to_string(), Some("Tampon".to_string()))), "une classe imbriquée nomme la sienne : {p:?}");
    assert!(p.contains(&("seule".to_string(), None)), "{p:?}");
}

#[test]
fn rust_un_bloc_impl_porte_le_nom_court_et_le_chemin_ecrit() {
    let src = "pub trait Outil { fn jouer(&self) -> i32; }\npub struct Foo<U> { u: U }\nimpl<U> Foo<U> {\n    pub fn neuf(u: U) -> Self { Foo { u } }\n}\nimpl<U> Outil for Foo<U> {\n    fn jouer(&self) -> i32 { 1 }\n}\npub mod a { pub struct Barre; }\nimpl crate::a::Barre {\n    pub fn b(&self) {}\n}\n";
    let s = scopes("lib.rs", src);
    let foo: Vec<_> = s.iter().filter(|(n, _, _)| n == "Foo").collect();
    assert_eq!(foo.len(), 3, "la struct et ses deux blocs s'appellent Foo, sans <U> : {s:?}");
    for m in ["neuf", "jouer"] {
        assert!(s.iter().any(|(n, p, c)| n == m && p.as_deref() == Some("Foo") && c.is_none()), "{m} a Foo pour parent : {s:?}");
    }
    assert!(s.iter().any(|(n, p, c)| n == "b" && p.as_deref() == Some("Barre") && c.as_deref() == Some("crate::a::Barre")), "{s:?}");
    assert!(s.iter().any(|(n, _, c)| n == "Barre" && c.as_deref() == Some("crate::a::Barre")), "le bloc dit le chemin du type qu'il prolonge : {s:?}");
}

#[test]
fn cpp_une_definition_hors_classe_garde_le_chemin_ecrit() {
    let src = "namespace kz { class Foo { public: void qux(); }; }\nclass Bar { public: void f(); };\nvoid kz::Foo::qux() {}\nvoid Bar::f() {}\n";
    let s = scopes("a.cpp", src);
    assert!(s.iter().any(|(n, p, c)| n == "qux" && p.as_deref() == Some("Foo") && c.as_deref() == Some("kz::Foo")), "{s:?}");
    assert!(s.iter().any(|(n, p, c)| n == "f" && p.as_deref() == Some("Bar") && c.is_none()), "un seul élément, pas de chemin : {s:?}");
}
