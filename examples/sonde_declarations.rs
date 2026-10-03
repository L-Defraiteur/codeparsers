//! Sonde : sur les fichiers C/C++ d'une liste, les scopes anonymes, les
//! membres déclarés (méthodes, fonctions), les références des scopes, et les
//! définitions dont le parent n'est pas dans le fichier.
//! `cargo run --release --example sonde_declarations -- <liste>`
use codeparsers::parallel::parser_worker::{parse_file, ParseFileTask};
use codeparsers::parallel::project_parser::detect_language_from_path;
use rayon::prelude::*;

fn main() {
    let liste = std::fs::read_to_string(std::env::args().nth(1).unwrap()).unwrap();
    let fichiers: Vec<String> = liste
        .lines()
        .filter_map(|l| l.strip_prefix("C\t").map(String::from))
        .filter(|p| [".cpp", ".cc", ".c", ".h", ".hpp", ".hxx", ".cxx"].iter().any(|e| p.ends_with(e)))
        .collect();
    let comptes: Vec<[usize; 6]> = fichiers
        .par_iter()
        .filter_map(|p| {
            let content = std::fs::read_to_string(p).ok()?;
            let task = ParseFileTask { file_path: p.clone(), content, language: detect_language_from_path(p)? };
            let a = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| parse_file(&task))).ok()?;
            let ici: std::collections::HashSet<&str> = a.scopes.iter().map(|s| s.name.as_str()).collect();
            let membres = |t: &str| a.scopes.iter().flat_map(|s| s.members.iter().flatten()).filter(|m| format!("{:?}", m.member_type) == t).count();
            Some([
                a.scopes.iter().filter(|s| s.name == "AnonymousFunction").count(),
                membres("Method"),
                membres("Function"),
                a.scopes.iter().map(|s| s.identifier_references.len()).sum(),
                a.scopes.iter().filter(|s| s.parent.as_deref().is_some_and(|p| !ici.contains(p))).count(),
                a.scopes.len(),
            ])
        })
        .collect();
    let mut t = [0usize; 6];
    for c in &comptes {
        for i in 0..6 {
            t[i] += c[i];
        }
    }
    println!("{} fichiers C/C++ : {} scopes, {} anonymes, membres Method {}, Function {}, références {}, parent ailleurs {}", comptes.len(), t[5], t[0], t[1], t[2], t[3], t[4]);
}
