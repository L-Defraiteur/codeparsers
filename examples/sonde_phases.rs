//! Sonde : où passe le temps d'un fichier — tree-sitter seul, `parse_file`
//! entier, `finalize` rejoué sur le résultat.
//! `cargo run --release --example sonde_phases -- <fichier.c|.rs|.hpp>`
use std::time::Instant;

use codeparsers::parallel::parser_worker::{finalize, parse_file, ParseFileTask};
use codeparsers::parallel::project_parser::detect_language_from_path;

fn main() {
    for f in std::env::args().skip(1) {
        let content = std::fs::read_to_string(&f).unwrap();
        let mut p = tree_sitter::Parser::new();
        let lang: tree_sitter::Language = if f.ends_with(".rs") {
            tree_sitter_rust::LANGUAGE.into()
        } else if f.ends_with(".c") {
            tree_sitter_c::LANGUAGE.into()
        } else {
            tree_sitter_cpp::LANGUAGE.into()
        };
        p.set_language(&lang).unwrap();
        let t = Instant::now();
        let arbre = p.parse(&content, None).unwrap();
        let ts = t.elapsed().as_secs_f64() * 1000.0;
        let task = ParseFileTask { file_path: f.clone(), content: content.clone(), language: detect_language_from_path(&f).unwrap() };
        let t = Instant::now();
        let mut a = parse_file(&task);
        let tout = t.elapsed().as_secs_f64() * 1000.0;
        let t = Instant::now();
        finalize(&mut a, &content);
        let fin = t.elapsed().as_secs_f64() * 1000.0;
        let refs: usize = a.scopes.iter().map(|s| s.identifier_references.len()).sum();
        println!("{f}\n  tree-sitter {ts:.0} ms ({} nœuds) ; parse_file {tout:.0} ms ; finalize {fin:.0} ms ; {} scopes, {refs} références", arbre.root_node().descendant_count(), a.scopes.len());
    }
}
