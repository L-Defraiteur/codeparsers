//! **`new` n'est pas un mot vide en Rust, en Python ni en C** (ticket du 11 octobre 2026) : la liste de
//! mots vides de ces langues le reprenait de JavaScript, où c'est un mot-clé ; `Inner::new()` n'était
//! jamais une référence, et `usages` d'un constructeur `new` ne rendait rien. Il reste vide en C++, en C# (un
//! mot-clé) et en Go (une fonction intégrée, `new(T)`).

use std::collections::HashMap;

use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};

fn references(fichier: &str, src: &str, scope: &str) -> Vec<String> {
    let chemin = format!("/virtual/{fichier}");
    let a = ProjectParser::new(ProjectParserOptions { verbose: false }).parse_project(ParseProjectOptions {
        root: "/virtual".to_string(),
        files: vec![chemin.clone()],
        content_map: Some(HashMap::from([(chemin, src.to_string())])),
        resolve_relationships: Some(false),
        resolver_options: None,
    });
    a.files
        .values()
        .flat_map(|fa| fa.scopes.iter())
        .filter(|s| s.name == scope)
        .flat_map(|s| s.identifier_references.iter().map(|r| r.identifier.clone()))
        .collect()
}

#[test]
fn rust_un_appel_a_new_est_une_reference() {
    let src = "pub struct Inner;\nimpl Inner {\n    pub fn new() -> Self { Inner }\n}\npub fn f() -> Inner {\n    Inner::new()\n}\n";
    let r = references("lib.rs", src, "f");
    assert!(r.iter().any(|i| i == "new"), "f appelle Inner::new : {r:?}");
}

#[test]
fn python_un_appel_a_new_est_une_reference() {
    let src = "class Fabrique:\n    @staticmethod\n    def new():\n        return Fabrique()\n\n\ndef f():\n    return Fabrique.new()\n";
    let r = references("a.py", src, "f");
    assert!(r.iter().any(|i| i == "new"), "f appelle Fabrique.new : {r:?}");
}

#[test]
fn c_un_appel_a_new_est_une_reference() {
    let src = "struct pile *new(void);\n\nstruct pile *f(void) {\n    return new();\n}\n";
    let r = references("a.c", src, "f");
    assert!(r.iter().any(|i| i == "new"), "f appelle new : {r:?}");
}
