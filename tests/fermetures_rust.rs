//! **T5 du B9 Rust** (session recherche, 11 octobre 2026) : l'appel à une
//! fermeture locale est relié à elle, et une fonction passée en argument est
//! un appel.
//!
//! `let double = |x| x * 2; … .map(|x| double(x))` : la fermeture a son
//! scope (`Lambda double`), mais `double(x)` était écarté (une locale cache
//! le nom nu). `map(aide)` reliait `aide` en « other ».

use std::collections::HashMap;

use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};
use codeparsers::relationship_resolution::types::RelationshipResolverOptions;

const SRC: &str = "pub fn aide(x: i32) -> i32 { x + 1 }\npub fn applique() -> i32 {\n    let double = |x: i32| x * 2;\n    let v: Vec<i32> = vec![1, 2].into_iter().map(aide).collect();\n    v.into_iter().map(|x| double(x)).sum()\n}\n";

/// (de, vers, genres d'usage des sites) des relations d'appel, fichier seul.
fn relations() -> Vec<(String, String, Vec<String>)> {
    let chemin = "/virtual/fermetures.rs".to_string();
    let a = ProjectParser::new(ProjectParserOptions { verbose: false }).parse_project(ParseProjectOptions {
        root: "/virtual".to_string(),
        files: vec![chemin.clone()],
        content_map: Some(HashMap::from([(chemin, SRC.to_string())])),
        resolve_relationships: Some(true),
        resolver_options: Some(RelationshipResolverOptions { resolve_cross_file: Some(false), ..Default::default() }),
    });
    let mut v: Vec<_> = a
        .relationships
        .expect("les relations")
        .relationships
        .iter()
        .filter(|x| format!("{:?}", x.r#type) == "CONSUMES")
        .map(|x| (x.from_name.clone(), x.to_name.clone(), x.metadata.as_ref().map(|m| m.sites.iter().map(|s| format!("{:?}", s.usage)).collect()).unwrap_or_default()))
        .collect();
    v.sort();
    v
}

#[test]
fn l_appel_a_une_fermeture_locale_est_relie_a_elle() {
    let r = relations();
    assert!(r.iter().any(|(de, vers, _)| vers == "double" && (de == "Closure" || de == "applique")), "double(x) relié à la fermeture : {r:?}");
}

#[test]
fn une_fonction_passee_en_argument_est_un_appel() {
    let r = relations();
    let aide: Vec<_> = r.iter().filter(|(de, vers, _)| de == "applique" && vers == "aide").collect();
    assert_eq!(aide.len(), 1, "{r:?}");
    assert!(aide[0].2.iter().any(|u| u == "Call"), "map(aide) : un appel, pas « other » — {r:?}");
}
