//! Sonde : le temps de `parse_file` sur des préfixes d'un même fichier
//! (1/8, 1/4, 1/2, entier) — linéaire, ou quadratique ?
//! `cargo run --release --example sonde_echelle -- <fichier>…`
use std::time::Instant;

use codeparsers::parallel::parser_worker::{parse_file, ParseFileTask};
use codeparsers::parallel::project_parser::detect_language_from_path;

fn main() {
    for f in std::env::args().skip(1) {
        let tout = std::fs::read_to_string(&f).unwrap();
        let lignes: Vec<&str> = tout.lines().collect();
        let language = detect_language_from_path(&f).unwrap();
        println!("== {f} ({} lignes)", lignes.len());
        for div in [8, 4, 2, 1] {
            let n = lignes.len() / div;
            let content = lignes[..n].join("\n");
            let task = ParseFileTask { file_path: f.clone(), content, language: language.clone() };
            let t = Instant::now();
            let a = parse_file(&task);
            println!("  1/{div} : {:>6} lignes {:>8.0} ms  {} scopes", n, t.elapsed().as_secs_f64() * 1000.0, a.scopes.len());
        }
    }
}
