//! **Une déclaration et sa définition sont une seule chose.**
//!
//! En C++, `class Foo { int bar(int x) const; };` dans foo.h et
//! `int Foo::bar(int x) const { … }` dans foo.cpp : la définition est le
//! scope ; la déclaration est une ligne de sa classe (`members`, avec sa
//! signature et sa ligne), pas une seconde chose — ni un scope, ni, comme
//! avant, des références « inconnues » portées par la classe jusqu'au nom
//! du paramètre. La définition hors classe garde le nom de sa classe en
//! parent, même écrite `kz::Foo::qux` ou `Foo::~Foo`.

use std::collections::HashMap;

use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};
use codeparsers::scope_extraction::types::{ClassMemberInfoMemberType, ScopeInfo};

const H: &str = "#pragma once\nnamespace kz {\nclass Foo {\npublic:\n    int bar(int x) const;\n    void baz();\n    ~Foo();\n    virtual int pure(int y) = 0;\n    int n;\n};\nint libre(int a);\n}\n";
const CPP: &str = "#include \"foo.h\"\nnamespace kz {\nint Foo::bar(int x) const {\n    return libre(x);\n}\nvoid Foo::baz() {\n    bar(1);\n}\nFoo::~Foo() {\n}\nint libre(int a) {\n    return a + 1;\n}\n}\nint kz::Foo::qux() {\n    return 2;\n}\n";

fn scopes(nom: &str, source: &str) -> Vec<ScopeInfo> {
    let chemin = format!("/virtual/{nom}");
    let a = ProjectParser::new(ProjectParserOptions { verbose: false }).parse_project(ParseProjectOptions {
        root: "/virtual".to_string(),
        files: vec![chemin.clone()],
        content_map: Some(HashMap::from([(chemin, source.to_string())])),
        resolve_relationships: Some(false),
        resolver_options: None,
    });
    a.files.into_values().next().unwrap().scopes
}

fn scope<'a>(v: &'a [ScopeInfo], nom: &str) -> &'a ScopeInfo {
    v.iter().find(|s| s.name == nom).unwrap_or_else(|| panic!("pas de scope {nom} : {:?}", v.iter().map(|s| &s.name).collect::<Vec<_>>()))
}

#[test]
fn une_methode_declaree_est_un_membre_de_sa_classe() {
    let h = scopes("foo.h", H);
    let foo = scope(&h, "Foo");
    let membres = foo.members.clone().unwrap_or_default();
    let bar = membres.iter().find(|m| m.name == "bar").unwrap_or_else(|| panic!("bar n'est pas un membre : {membres:#?}"));
    assert_eq!(bar.member_type, ClassMemberInfoMemberType::Method);
    assert_eq!(bar.line, 5);
    assert_eq!(bar.signature.as_deref(), Some("int bar(int x) const"));
    for nom in ["baz", "~Foo", "pure"] {
        assert!(membres.iter().any(|m| m.name == nom && m.member_type == ClassMemberInfoMemberType::Method), "{nom} : {membres:#?}");
    }
    assert!(membres.iter().any(|m| m.name == "n" && m.member_type == ClassMemberInfoMemberType::Property), "{membres:#?}");
    // Pas de second scope pour une déclaration.
    assert!(!h.iter().any(|s| s.name == "bar"), "une déclaration n'est pas un scope");
}

#[test]
fn une_declaration_n_est_pas_une_reference_de_sa_classe() {
    let h = scopes("foo.h", H);
    let refs: Vec<&str> = scope(&h, "Foo").identifier_references.iter().map(|r| r.identifier.as_str()).collect();
    for nom in ["bar", "baz", "x", "y", "pure"] {
        assert!(!refs.contains(&nom), "{nom} est déclaré, pas utilisé : {refs:?}");
    }
}

#[test]
fn une_fonction_libre_declaree_est_un_membre_de_son_namespace() {
    let h = scopes("foo.h", H);
    let kz = scope(&h, "kz");
    let membres = kz.members.clone().unwrap_or_default();
    let libre = membres.iter().find(|m| m.name == "libre").unwrap_or_else(|| panic!("{membres:#?}"));
    assert_eq!((libre.member_type.clone(), libre.line), (ClassMemberInfoMemberType::Function, 11));
    let refs: Vec<&str> = kz.identifier_references.iter().map(|r| r.identifier.as_str()).collect();
    assert!(!refs.contains(&"libre") && !refs.contains(&"a"), "{refs:?}");
}

#[test]
fn une_definition_hors_classe_garde_sa_classe() {
    let c = scopes("foo.cpp", CPP);
    for (nom, parent) in [("bar", "Foo"), ("baz", "Foo"), ("~Foo", "Foo"), ("qux", "Foo"), ("libre", "kz")] {
        let s = scope(&c, nom);
        assert_eq!(s.parent.as_deref(), Some(parent), "{nom}");
    }
    assert!(!c.iter().any(|s| s.name == "AnonymousFunction"), "{:?}", c.iter().map(|s| &s.name).collect::<Vec<_>>());
}
