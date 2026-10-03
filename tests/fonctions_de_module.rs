//! **Une fonction déclarée dans un `mod` Rust est un scope.**
//!
//! L'extracteur ne prenait une fonction libre que sans parent, pour écarter
//! les fonctions imbriquées dans un corps ; il écartait du même coup toutes
//! celles d'un module — dont les 348 `#[test]` de `src/dataflow` (3 octobre
//! 2026). Une fonction imbriquée dans le corps d'une autre reste écartée.

use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};
use codeparsers::scope_extraction::types::ScopeInfo;
use std::collections::HashMap;

fn scopes(nom: &str, source: &str) -> Vec<ScopeInfo> {
    let chemin = format!("/virtual/{nom}");
    let mut a = ProjectParser::new(ProjectParserOptions { verbose: false }).parse_project(ParseProjectOptions {
        root: "/virtual".to_string(),
        files: vec![chemin.clone()],
        content_map: Some(HashMap::from([(chemin.clone(), source.to_string())])),
        resolve_relationships: Some(false),
        resolver_options: None,
    });
    a.files.remove(&chemin).expect("le fichier analysé").scopes
}

const SRC: &str = r#"pub fn top() -> u32 { 0 }

mod outils {
    pub fn plain() -> u32 {
        fn imbriquee() -> u32 { 1 }
        imbriquee()
    }

    pub mod profond {
        pub fn tout_au_fond() {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn helper() -> u32 { 2 }

    #[test]
    fn adds() {
        assert_eq!(top(), helper() - 2);
    }
}
"#;

#[test]
fn les_fonctions_d_un_module_sont_des_scopes_avec_leur_module_pour_parent() {
    let s = scopes("lib.rs", SRC);
    let parent = |nom: &str| {
        s.iter().find(|x| x.name == nom).unwrap_or_else(|| panic!("pas de scope {nom} ; scopes : {:?}", s.iter().map(|x| &x.name).collect::<Vec<_>>())).parent.clone()
    };
    assert_eq!(parent("plain"), Some("outils".to_string()));
    assert_eq!(parent("tout_au_fond"), Some("profond".to_string()));
    assert_eq!(parent("helper"), Some("tests".to_string()));
    assert_eq!(parent("adds"), Some("tests".to_string()));
    assert_eq!(parent("top"), None);
}

#[test]
fn une_fonction_imbriquee_dans_un_corps_reste_ecartee() {
    let s = scopes("lib.rs", SRC);
    assert!(!s.iter().any(|x| x.name == "imbriquee"), "comme avant, une fonction dans un corps n'est pas un scope");
}

#[test]
fn une_fonction_de_module_porte_ses_propres_references() {
    let s = scopes("lib.rs", SRC);
    let adds = s.iter().find(|x| x.name == "adds").expect("adds");
    let noms: Vec<&str> = adds.identifier_references.iter().map(|r| r.identifier.as_str()).collect();
    assert!(noms.contains(&"top") && noms.contains(&"helper"), "références de adds : {noms:?}");
}

#[test]
fn le_parent_d_une_methode_est_son_impl_pas_la_struct_homonyme() {
    // `struct Sq;` puis `impl Shape for Sq { fn area… }` : deux scopes `Sq`
    // dans le fichier ; le parent de `area` est celui qui le contient
    // (l'impl), pas le premier homonyme (la struct). Sans cela, l'arête
    // `HAS_PARENT` de toute méthode Rust visait la struct, et l'impl — qui
    // porte l'`IMPLEMENTS` — n'avait plus d'enfants.
    let src = "pub trait Shape {\n    fn area(&self) -> f64;\n}\n\npub struct Sq;\n\nimpl Shape for Sq {\n    fn area(&self) -> f64 {\n        1.0\n    }\n}\n";
    let mut contenus = HashMap::new();
    contenus.insert("/virtual/sq.rs".to_string(), src.to_string());
    let res = ProjectParser::new(ProjectParserOptions { verbose: false })
        .parse_project(ParseProjectOptions {
            root: "/virtual".to_string(),
            files: vec!["/virtual/sq.rs".into()],
            content_map: Some(contenus),
            resolve_relationships: Some(true),
            resolver_options: None,
        })
        .relationships
        .unwrap();
    let parents: Vec<(usize, usize)> = res
        .relationships
        .iter()
        .filter(|r| r.r#type == codeparsers::relationship_resolution::types::RelationshipType::PARENTOF && r.to_name == "area" && r.from_name == "Sq")
        .map(|r| {
            let e = &res.uuid_mapping[&r.from_uuid];
            (e.start_line, e.end_line)
        })
        .collect();
    assert_eq!(parents, vec![(7, 11)], "le parent de area est l'impl (lignes 7-11), pas la struct (ligne 5)");
}
