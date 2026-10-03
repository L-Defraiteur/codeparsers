//! Les références et les imports de chaque scope des fichiers donnés.
//! `cargo run --example voir_refs -- <fichier>…`
use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};

fn main() {
    let files: Vec<String> = std::env::args().skip(1).collect();
    let root = std::path::Path::new(&files[0]).parent().unwrap().to_string_lossy().to_string();
    let r = ProjectParser::new(ProjectParserOptions { verbose: false }).parse_project(ParseProjectOptions {
        root,
        files,
        content_map: None,
        resolve_relationships: Some(false),
        resolver_options: None,
    });
    for (f, a) in &r.files {
        println!("== {f}");
        for s in &a.scopes {
            println!("  scope {} [{:?}]", s.name, s.r#type);
            for i in &s.import_references {
                println!("    import source={} imported={} alias={:?} local={}", i.source, i.imported, i.alias, i.is_local);
            }
            for i in &s.identifier_references {
                println!("    ref {} q={:?} kind={:?} source={:?} l{}", i.identifier, i.qualifier, i.kind, i.source, i.line);
            }
        }
    }
}
