//! Un même fichier analysé `n` fois en parallèle : écrit chaque sortie
//! (JSON) dans `<dossier>/<i>.json` — pour trouver ce qui varie.
//! `cargo run --release --example voir_json_par -- <fichier> <n> <dossier>`
use codeparsers::parallel::parser_worker::{parse_file, ParseFileTask};
use codeparsers::parallel::project_parser::detect_language_from_path;
use rayon::prelude::*;

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let content = std::fs::read_to_string(&a[1]).unwrap();
    let n: usize = a[2].parse().unwrap();
    (0..n).into_par_iter().for_each(|i| {
        let task = ParseFileTask { file_path: a[1].clone(), content: content.clone(), language: detect_language_from_path(&a[1]).unwrap() };
        std::fs::write(format!("{}/{i}.json", a[3]), serde_json::to_string_pretty(&parse_file(&task)).unwrap()).unwrap();
    });
}
