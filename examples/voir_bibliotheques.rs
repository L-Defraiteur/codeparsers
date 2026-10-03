//! Les bibliothèques (USES_LIBRARY) de fichiers analysés en fichier seul,
//! avec un exemple de site : `cargo run --example voir_bibliotheques -- <fichier>…`
use std::collections::BTreeMap;

use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};
use codeparsers::relationship_resolution::types::{RelationshipResolverOptions, RelationshipType};

fn main() {
    let files: Vec<String> = std::env::args().skip(1).collect();
    let root = std::path::Path::new(&files[0]).parent().unwrap().to_string_lossy().to_string();
    let r = ProjectParser::new(ProjectParserOptions { verbose: false }).parse_project(ParseProjectOptions {
        root,
        files: files.clone(),
        content_map: None,
        resolve_relationships: Some(true),
        resolver_options: Some(RelationshipResolverOptions {
            include_file_level_refs: Some(false),
            include_child_refs: Some(false),
            resolve_cross_file: Some(false),
            // `SANS_PROJET=1` : comme l'appel unique de rag3weaver, sans liste.
            project_files: if std::env::var("SANS_PROJET").is_ok() { None } else { Some(files) },
            ..Default::default()
        }),
    });
    let mut libs: BTreeMap<String, (usize, String)> = BTreeMap::new();
    let rels = r.relationships.unwrap();
    let mut table: Vec<&String> = rels.external_libraries.keys().collect();
    table.sort();
    println!("external_libraries ({}) : {:?}", table.len(), table);
    for x in rels.relationships.clone() {
        if x.r#type == RelationshipType::USESLIBRARY {
            let e = libs.entry(x.to_name.clone()).or_insert((0, format!("{}:{}", x.from_file, x.metadata.as_ref().and_then(|m| m.sites.first().and_then(|s| s.line)).unwrap_or(0))));
            e.0 += 1;
        }
    }
    for (l, (n, site)) in libs {
        println!("{l:<24} {n:>4}  {site}");
    }
}
