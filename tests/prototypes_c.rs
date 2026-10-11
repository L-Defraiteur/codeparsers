//! **Les prototypes C au niveau du fichier sont des déclarations, pas des
//! références** (ticket du 4 octobre, P1 de la stèle du produit). Un
//! prototype hors de tout namespace (`int libre(int a);`, la moitié des
//! en-têtes C) tombait dans le scope de fichier, dont les références sont
//! lues sur le texte : `libre` et `a` y devenaient des références par le nom.
//! Le prototype est un membre du scope de fichier (genre `Function`) ; son
//! nom et ses paramètres ne sont pas des références, les types qu'il cite le
//! restent. Et un `struct Point` sans corps (dans un paramètre) n'est pas une
//! seconde définition de `Point`.

use std::collections::HashMap;

use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};
use codeparsers::scope_extraction::types::{ScopeInfo, ScopeInfoType};

fn scopes(fichier: &str, source: &str) -> Vec<ScopeInfo> {
    let chemin = format!("/virtual/{fichier}");
    let a = ProjectParser::new(ProjectParserOptions { verbose: false }).parse_project(ParseProjectOptions {
        root: "/virtual".to_string(),
        files: vec![chemin.clone()],
        content_map: Some(HashMap::from([(chemin, source.to_string())])),
        resolve_relationships: Some(false),
        resolver_options: None,
    });
    a.files.into_values().flat_map(|fa| fa.scopes).collect()
}

fn fichier_de(s: &[ScopeInfo]) -> (Vec<String>, Vec<String>) {
    let membres = s
        .iter()
        .filter(|x| x.r#type == ScopeInfoType::Module)
        .flat_map(|x| x.members.iter().flatten().map(|m| m.name.clone()))
        .collect();
    let refs = s.iter().filter(|x| x.r#type == ScopeInfoType::Module).flat_map(|x| x.identifier_references.iter().map(|r| r.identifier.clone())).collect();
    (membres, refs)
}

const EN_TETE: &str = "#ifndef FOO_H\n#define FOO_H\nint libre(int a);\nstruct Point { int x; };\ndouble aire(const struct Point* p, int n);\n#ifdef __cplusplus\nextern \"C\" {\n#endif\nvoid relache(void* tampon);\n#ifdef __cplusplus\n}\n#endif\n#endif\n";

#[test]
fn un_prototype_c_est_un_membre_pas_une_reference() {
    let s = scopes("foo.h", EN_TETE);
    let (membres, refs) = fichier_de(&s);
    for nom in ["libre", "aire", "relache"] {
        assert!(membres.contains(&nom.to_string()), "{nom} membre du fichier : {membres:?}");
    }
    for nom in ["libre", "a", "aire", "p", "n", "relache", "tampon"] {
        assert!(!refs.contains(&nom.to_string()), "{nom} n'est pas une référence : {refs:?}");
    }
}

#[test]
fn un_struct_sans_corps_n_est_pas_une_definition() {
    let s = scopes("foo.h", EN_TETE);
    let points = s.iter().filter(|x| x.name == "Point").count();
    assert_eq!(points, 1, "une seule définition de Point");
}

#[test]
fn le_meme_en_cpp() {
    let s = scopes("foo.hpp", "#pragma once\nint libre(int a);\nclass Avance;\nclass Avance { int x; };\n");
    let (membres, refs) = fichier_de(&s);
    assert!(membres.contains(&"libre".to_string()), "{membres:?}");
    assert!(!refs.contains(&"libre".to_string()) && !refs.contains(&"a".to_string()), "{refs:?}");
    assert_eq!(s.iter().filter(|x| x.name == "Avance").count(), 1, "une déclaration anticipée n'est pas une définition");
}
