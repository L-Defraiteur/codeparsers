//! Sonde : le temps de `parse_file`, fichier par fichier, sur la liste de
//! `sonde_analyse` — pour nommer les retardataires d'un lot.
//!
//! `cargo run --release --example sonde_fichiers -- <liste> [lot]`
//! Rend les fichiers les plus lents (ms, Ko, langage, scopes), la somme, et
//! ce que coûterait chaque forme : par lots de `lot` lignes (le plus lent de
//! chaque lot fixe sa durée, au mieux), ou en un seul appel.

use std::time::Instant;

use codeparsers::parallel::parser_worker::{parse_file, ParseFileTask};
use codeparsers::parallel::project_parser::detect_language_from_path;
use rayon::prelude::*;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let liste = std::fs::read_to_string(&args[1]).expect("liste");
    let lot: usize = args.get(2).and_then(|v| v.parse().ok()).unwrap_or(64);
    let lignes: Vec<(bool, String)> = liste
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| match l.split_once('\t') {
            Some((m, p)) => (m == "C", p.to_string()),
            None => (true, l.to_string()),
        })
        .collect();
    // (rang dans la liste, chemin, ms, octets, scopes)
    let mesures: Vec<(usize, String, f64, usize, usize)> = lignes
        .par_iter()
        .enumerate()
        .filter(|(_, (c, _))| *c)
        .filter_map(|(i, (_, p))| {
            let content = std::fs::read_to_string(p).ok()?;
            let language = detect_language_from_path(p)?;
            let task = ParseFileTask { file_path: p.clone(), content, language };
            let t = Instant::now();
            let a = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| parse_file(&task))).ok()?;
            Some((i, p.clone(), t.elapsed().as_secs_f64() * 1000.0, task.content.len(), a.scopes.len()))
        })
        .collect();
    let somme: f64 = mesures.iter().map(|m| m.2).sum();
    let mut tri = mesures.clone();
    tri.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap());
    println!("{} fichiers, somme {:.1} s (CPU d'analyse)", mesures.len(), somme / 1000.0);
    let mut cumul = 0.0;
    for (n, m) in tri.iter().enumerate() {
        cumul += m.2;
        if n < 25 {
            let court = m.1.split("/rag3db/").nth(1).unwrap_or(&m.1);
            println!("{:>8.0} ms {:>7.0} Ko {:>6} scopes  {court}", m.2, m.3 as f64 / 1024.0, m.4);
        }
        if [10, 50, 100].contains(&(n + 1)) {
            println!("   … les {} plus lents : {:.0} % du total", n + 1, cumul * 100.0 / somme);
        }
    }
    // Par lots : chaque lot dure au moins son plus lent, et au moins sa
    // somme répartie sur les fils.
    let fils = rayon::current_num_threads() as f64;
    let mut borne_lots = 0.0;
    for debut in (0..lignes.len()).step_by(lot.max(1)) {
        let du_lot: Vec<f64> = mesures.iter().filter(|m| m.0 >= debut && m.0 < debut + lot).map(|m| m.2).collect();
        if du_lot.is_empty() {
            continue;
        }
        let max = du_lot.iter().cloned().fold(0.0, f64::max);
        borne_lots += max.max(du_lot.iter().sum::<f64>() / fils);
    }
    let max_global = tri.first().map(|m| m.2).unwrap_or(0.0);
    println!(
        "borne basse par lots de {lot} : {:.1} s ; en un appel : {:.1} s (le plus lent {:.1} s, somme / {fils} fils {:.1} s)",
        borne_lots / 1000.0,
        max_global.max(somme / fils) / 1000.0,
        max_global / 1000.0,
        somme / fils / 1000.0
    );
}
