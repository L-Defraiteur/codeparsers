//! **Probe: the time of the analysis of a tree** (19 November 2026). `PROBE_DIR=<dir>`: parses every source file of
//! the tree (no relationship resolution) and prints the files, the scopes and the milliseconds. A probe, ignored; it
//! judges nothing.
//!
//! Run with: PROBE_DIR=… cargo test --release --test probe_parse_time -- --ignored --nocapture

use std::collections::HashMap;
use std::time::Instant;

use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};

fn files(d: &std::path::Path, v: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = std::fs::read_dir(d) else { return };
    for e in entries.flatten() {
        let p = e.path();
        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name.starts_with('.') || name == "target" || name == "node_modules" || name == "build" {
            continue;
        }
        if p.is_dir() {
            files(&p, v)
        } else if p.extension().and_then(|x| x.to_str()).is_some_and(|x| ["rs", "c", "h", "cpp", "cc", "hpp", "py", "ts", "js", "go", "cs"].contains(&x)) {
            v.push(p)
        }
    }
}

#[test]
#[ignore]
fn the_time_of_the_analysis_of_a_tree() {
    let dir = std::env::var("PROBE_DIR").expect("PROBE_DIR");
    let mut v = Vec::new();
    files(std::path::Path::new(&dir), &mut v);
    v.sort();
    let content: HashMap<String, String> = v.iter().filter_map(|p| Some((p.display().to_string(), std::fs::read_to_string(p).ok()?))).collect();
    let t = Instant::now();
    let a = ProjectParser::new(ProjectParserOptions { verbose: false }).parse_project(ParseProjectOptions {
        root: dir.clone(),
        files: content.keys().cloned().collect(),
        content_map: Some(content),
        resolve_relationships: Some(false),
        resolver_options: None,
    });
    let scopes: usize = a.files.values().map(|f| f.scopes.len()).sum();
    println!("PROBE {dir} : files {} ; scopes {scopes} ; parse {} ms", a.files.len(), t.elapsed().as_millis());
}
