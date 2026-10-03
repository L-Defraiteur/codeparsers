//! Sonde : l'empreinte de la sortie de `parse_file` (sérialisée en JSON
//! entier, blake3), fichier par fichier, sur une liste de `sonde_analyse` —
//! pour prouver qu'une optimisation ne change rien, champ à champ. Les
//! références d'un scope sortent dans un ordre qui varie d'une exécution à
//! l'autre (même binaire) : elles sont triées avant le hachage.
//! `cargo run --release --example sonde_empreinte -- <liste> <sortie.tsv>`
use std::time::Instant;

use codeparsers::parallel::parser_worker::{parse_file, ParseFileTask};
use codeparsers::parallel::project_parser::detect_language_from_path;
use rayon::prelude::*;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let liste = std::fs::read_to_string(&args[1]).expect("liste");
    let fichiers: Vec<String> = liste
        .lines()
        .filter_map(|l| match l.split_once('\t') {
            Some(("C", p)) => Some(p.to_string()),
            Some(_) => None,
            None => (!l.is_empty()).then(|| l.to_string()),
        })
        .collect();
    let t = Instant::now();
    let mut lignes: Vec<String> = fichiers
        .par_iter()
        .filter_map(|p| {
            let content = std::fs::read_to_string(p).ok()?;
            let task = ParseFileTask { file_path: p.clone(), content, language: detect_language_from_path(p)? };
            let a = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| parse_file(&task))).ok();
            let json = a.map(|a| canonique(serde_json::to_value(&a).unwrap())).unwrap_or_else(|| "panique".into());
            Some(format!("{p}\t{}", blake3::hash(json.as_bytes()).to_hex()))
        })
        .collect();
    lignes.sort();
    std::fs::write(&args[2], lignes.join("\n")).unwrap();
    println!("{} empreintes en {:.1} s → {}", lignes.len(), t.elapsed().as_secs_f64(), args[2]);
}

fn canonique(mut v: serde_json::Value) -> String {
    for champ in ["imports", "dependencies", "exports", "import_references"] {
        if let Some(l) = v.get_mut(champ).and_then(|l| l.as_array_mut()) {
            l.sort_by_key(|x| x.to_string());
        }
    }
    if let Some(scopes) = v.get_mut("scopes").and_then(|s| s.as_array_mut()) {
        for s in scopes {
            for champ in ["identifier_references", "import_references"] {
                if let Some(l) = s.get_mut(champ).and_then(|l| l.as_array_mut()) {
                    l.sort_by_key(|x| x.to_string());
                }
            }
        }
    }
    v.to_string()
}
