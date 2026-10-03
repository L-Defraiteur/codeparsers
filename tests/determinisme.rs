//! **Deux analyses du même code rendent les mêmes relations.**
//!
//! La table des homonymes (`nom → scopes`) se remplissait dans l'ordre d'une
//! `HashMap` de fichiers, qui change à chaque exécution ; le résolveur prend
//! souvent le premier candidat. Sur `src/dataflow`, deux passes identiques
//! différaient de 1 352 à 1 708 relations (3 octobre 2026).

use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};
use std::collections::HashMap;

fn relations(fichiers: &[(&str, &str)]) -> Vec<String> {
    let mut contenus = HashMap::new();
    let mut chemins = Vec::new();
    for (nom, source) in fichiers {
        let chemin = format!("/virtual/{nom}");
        contenus.insert(chemin.clone(), source.to_string());
        chemins.push(chemin);
    }
    let mut rels: Vec<String> = ProjectParser::new(ProjectParserOptions { verbose: false })
        .parse_project(ParseProjectOptions {
            root: "/virtual".to_string(),
            files: chemins,
            content_map: Some(contenus),
            resolve_relationships: Some(true),
            resolver_options: None,
        })
        .relationships
        .map_or_else(Vec::new, |r| r.relationships)
        .into_iter()
        .map(|r| format!("{:?} {} {} → {} {}", r.r#type, r.from_file, r.from_uuid, r.to_file, r.to_uuid))
        .collect();
    rels.sort();
    rels
}

#[test]
fn un_nom_defini_dans_plusieurs_fichiers_se_resout_toujours_pareil() {
    // `helper` est défini dans six fichiers ; `run` l'appelle sans import.
    let mut fichiers: Vec<(String, String)> = (0..6)
        .map(|i| (format!("m{i}.rs"), format!("pub fn helper() -> u32 {{ {i} }}\n")))
        .collect();
    fichiers.push(("run.rs".to_string(), "fn run() {\n    helper();\n}\n".to_string()));
    let refs: Vec<(&str, &str)> = fichiers.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
    let premiere = relations(&refs);
    for passe in 1..12 {
        assert_eq!(relations(&refs), premiere, "la passe {passe} diffère de la première");
    }
}
