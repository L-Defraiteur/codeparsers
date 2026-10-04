//! Sonde : en C++, combien de scopes ont un parent (`Foo::bar` défini hors de
//! sa classe) qui n'est pas défini dans le même fichier, et combien de
//! références « inconnues » un en-tête porte pour ses seules déclarations.
//! `cargo run --release --example sonde_cpp_hors_classe -- <liste>`
use std::collections::{HashMap, HashSet};

use codeparsers::parallel::parser_worker::{parse_file, ParseFileTask};
use codeparsers::parallel::project_parser::detect_language_from_path;
use rayon::prelude::*;

fn main() {
    let liste = std::fs::read_to_string(std::env::args().nth(1).unwrap()).unwrap();
    let fichiers: Vec<String> = liste
        .lines()
        .filter_map(|l| l.strip_prefix("C\t").map(String::from))
        .filter(|p| [".cpp", ".cc", ".h", ".hpp", ".hxx", ".cxx"].iter().any(|e| p.ends_with(e)))
        .collect();
    let res: Vec<(String, Vec<(String, String, String)>)> = fichiers
        .par_iter()
        .filter_map(|p| {
            let content = std::fs::read_to_string(p).ok()?;
            let task = ParseFileTask { file_path: p.clone(), content, language: detect_language_from_path(p)? };
            let a = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| parse_file(&task))).ok()?;
            let ici: HashSet<String> = a.scopes.iter().map(|s| s.name.clone()).collect();
            let hors: Vec<(String, String, String)> = a
                .scopes
                .iter()
                .filter(|s| s.parent.as_ref().is_some_and(|par| !ici.contains(par)))
                .map(|s| (s.name.clone(), s.parent.clone().unwrap(), format!("{:?}", s.r#type)))
                .collect();
            Some((p.clone(), hors))
        })
        .collect();
    let total: usize = res.iter().map(|(_, h)| h.len()).sum();
    let mut par_type: HashMap<String, usize> = HashMap::new();
    for (_, h) in &res {
        for (_, _, t) in h {
            *par_type.entry(t.clone()).or_default() += 1;
        }
    }
    println!("{} fichiers C/C++ ; {} scopes dont le parent n'est pas dans le fichier : {:?}", res.len(), total, par_type);
    for (p, h) in res.iter().filter(|(_, h)| !h.is_empty()).take(5) {
        println!("  {} : {:?}", p.rsplit("/rag3db/").next().unwrap_or(p), &h[..h.len().min(4)]);
    }
}
