//! Sonde : `parse_project` sur une liste de fichiers, comme rag3weaver
//! l'appelle (mêmes options), en un appel ou par lots.
//!
//! `cargo run --release --example sonde_analyse -- <liste> <racine> <lot>`
//! — `lot` = 0 pour un seul appel, 64 pour la forme de l'index d'aujourd'hui.
//! Rend : lecture, analyse par fichier (parallèle), résolution, temps CPU,
//! pic de mémoire (VmHWM), et combien de fils travaillent, phase par phase
//! (échantillons de /proc/self/task toutes les 50 ms).

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};
use codeparsers::relationship_resolution::types::RelationshipResolverOptions;

fn ticks_par_fil() -> HashMap<String, u64> {
    let mut out = HashMap::new();
    let Ok(dir) = std::fs::read_dir("/proc/self/task") else { return out };
    for e in dir.flatten() {
        let Ok(stat) = std::fs::read_to_string(e.path().join("stat")) else { continue };
        // Après la parenthèse fermante du nom : champs 14 et 15 (utime, stime).
        let Some(fin) = stat.rfind(')') else { continue };
        let champs: Vec<&str> = stat[fin + 2..].split_whitespace().collect();
        let t = champs.get(11).and_then(|v| v.parse::<u64>().ok()).unwrap_or(0) + champs.get(12).and_then(|v| v.parse::<u64>().ok()).unwrap_or(0);
        out.insert(e.file_name().to_string_lossy().to_string(), t);
    }
    out
}

fn statut(cle: &str) -> String {
    std::fs::read_to_string("/proc/self/status")
        .unwrap_or_default()
        .lines()
        .find(|l| l.starts_with(cle))
        .map(|l| l[cle.len()..].trim().to_string())
        .unwrap_or_default()
}

fn cpu_s() -> f64 {
    ticks_par_fil().values().sum::<u64>() as f64 / 100.0
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let liste = std::fs::read_to_string(&args[1]).expect("liste");
    let racine = args[2].clone();
    let lot: usize = args.get(3).and_then(|v| v.parse().ok()).unwrap_or(0);
    // `C<tab>chemin` : du code ; `T<tab>chemin` : retenu par l'index mais
    // pas du code — il compte dans le découpage en lots, comme dans
    // `analyze_with`, puis il est écarté.
    let lignes: Vec<(bool, String)> = liste
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| match l.split_once('\t') {
            Some((m, p)) => (m == "C", p.to_string()),
            None => (true, l.to_string()),
        })
        .collect();
    let fichiers: Vec<String> = lignes.iter().filter(|(c, _)| *c).map(|(_, p)| p.clone()).collect();

    // L'échantillonneur : (instant en ms, fils dont le temps CPU a avancé).
    let serie: Arc<Mutex<Vec<(u128, usize)>>> = Arc::default();
    let fini = Arc::new(AtomicBool::new(false));
    let t0 = Instant::now();
    let ech = {
        let (serie, fini) = (serie.clone(), fini.clone());
        std::thread::spawn(move || {
            let mut avant = ticks_par_fil();
            while !fini.load(Ordering::Relaxed) {
                std::thread::sleep(Duration::from_millis(50));
                let maintenant = ticks_par_fil();
                let actifs = maintenant.iter().filter(|(k, v)| avant.get(*k).is_some_and(|a| *v > a)).count();
                serie.lock().unwrap().push((t0.elapsed().as_millis(), actifs));
                avant = maintenant;
            }
        })
    };

    let t = Instant::now();
    let mut contenus = HashMap::new();
    let mut octets = 0usize;
    for f in &fichiers {
        if let Ok(c) = std::fs::read_to_string(f) {
            octets += c.len();
            contenus.insert(f.clone(), c);
        }
    }
    let lecture_ms = t.elapsed().as_millis();

    let parser = ProjectParser::new(ProjectParserOptions { verbose: false });
    let lots: Vec<Vec<String>> = if lot == 0 {
        vec![fichiers.clone()]
    } else {
        lignes.chunks(lot).map(|c| c.iter().filter(|(code, _)| *code).map(|(_, p)| p.clone()).collect()).filter(|v: &Vec<String>| !v.is_empty()).collect()
    };
    let (mut parse_ms, mut rel_ms, mut relations, mut scopes) = (0u128, 0u128, 0usize, 0usize);
    // Les bornes de phase, pour lire la série : (début, fin, phase).
    let mut bornes: Vec<(u128, u128, &str)> = Vec::new();
    let cpu0 = cpu_s();
    let t = Instant::now();
    for l in &lots {
        let map: HashMap<String, String> = l.iter().filter_map(|f| contenus.get(f).map(|c| (f.clone(), c.clone()))).collect();
        let debut = t0.elapsed().as_millis();
        let r = parser.parse_project(ParseProjectOptions {
            root: racine.clone(),
            files: l.clone(),
            content_map: Some(map),
            resolve_relationships: Some(true),
            resolver_options: Some(RelationshipResolverOptions {
                include_file_level_refs: Some(false),
                include_child_refs: Some(false),
                ..Default::default()
            }),
        });
        let fin = t0.elapsed().as_millis();
        let p = r.stats.parse_time_ms;
        let rr = r.stats.relationship_time_ms.unwrap_or(0);
        // L'analyse finit où la résolution commence ; ce qui reste avant est
        // la copie des contenus et la préparation des tâches.
        bornes.push((fin - rr - p, fin - rr, "analyse"));
        bornes.push((fin - rr, fin, "résolution"));
        let _ = debut;
        parse_ms += p;
        rel_ms += rr;
        scopes += r.stats.total_scopes;
        relations += r.relationships.as_ref().map(|x| x.relationships.len()).unwrap_or(0);
    }
    let total_ms = t.elapsed().as_millis();
    let cpu = cpu_s() - cpu0;
    fini.store(true, Ordering::Relaxed);
    ech.join().unwrap();

    let serie = serie.lock().unwrap();
    let moyenne = |phase: &str| {
        let v: Vec<usize> = serie.iter().filter(|(t, _)| bornes.iter().any(|(a, b, p)| *p == phase && t >= a && t <= b)).map(|(_, n)| *n).collect();
        if v.is_empty() {
            return (0.0, 0usize, 0usize);
        }
        let un = v.iter().filter(|n| **n <= 1).count();
        (v.iter().sum::<usize>() as f64 / v.len() as f64, un * 100 / v.len(), v.len())
    };
    let (ma, una, na) = moyenne("analyse");
    let (mr, unr, nr) = moyenne("résolution");
    println!(
        "fichiers {} ({:.1} Mo), lots {} × {}\nlecture {lecture_ms} ms\nappels {total_ms} ms : analyse {parse_ms} ms, résolution {rel_ms} ms, autre {} ms\nCPU {cpu:.1} s ({:.1} cœurs en moyenne)\nfils actifs : analyse {ma:.1} (≤1 fil {una} % du temps, {na} éch.), résolution {mr:.1} (≤1 fil {unr} %, {nr} éch.)\nscopes {scopes}, relations {relations}\nVmHWM {}  fils rayon {}",
        fichiers.len(),
        octets as f64 / 1e6,
        lots.len(),
        if lot == 0 { fichiers.len() } else { lot },
        total_ms - parse_ms - rel_ms,
        cpu / (total_ms as f64 / 1000.0),
        statut("VmHWM:"),
        rayon::current_num_threads(),
    );
}
