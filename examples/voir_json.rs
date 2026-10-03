//! La sortie entière de `parse_file` pour un fichier, en JSON indenté.
//! `cargo run --release --example voir_json -- <fichier>`
use codeparsers::parallel::parser_worker::{parse_file, ParseFileTask};
use codeparsers::parallel::project_parser::detect_language_from_path;

fn main() {
    let p = std::env::args().nth(1).unwrap();
    let content = std::fs::read_to_string(&p).unwrap();
    let task = ParseFileTask { file_path: p.clone(), content, language: detect_language_from_path(&p).unwrap() };
    println!("{}", serde_json::to_string_pretty(&parse_file(&task)).unwrap());
}
