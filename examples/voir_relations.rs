//! Les relations d'un ou plusieurs fichiers, en mode « fichier seul », dont
//! un bout porte le nom donné : `cargo run --example voir_relations -- <nom> <fichier>…`
use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};
use codeparsers::relationship_resolution::types::RelationshipResolverOptions;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let nom = &a[1];
    let files: Vec<String> = a[2..].to_vec();
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
            project_files: Some(files),
            ..Default::default()
        }),
    });
    let mut v: Vec<String> = r
        .relationships
        .map(|x| x.relationships)
        .unwrap_or_default()
        .iter()
        .filter(|r| &r.from_name == nom || &r.to_name == nom)
        .map(|r| format!("{:?} {} → {} sites {:?}", r.r#type, r.from_name, r.to_name, r.metadata.as_ref().map(|m| m.sites.iter().map(|s| s.line).collect::<Vec<_>>())))
        .collect();
    v.sort();
    println!("{}", v.join("\n"));
}
