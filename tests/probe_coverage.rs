//! **La sonde de la couverture** (lot 1 de l'intra-scope) : sur les dossiers de `SONDE_DIRS` (séparés par des
//! virgules), la part de chaque catégorie de nœud par langue, et les sortes les plus fréquentes parmi les identifiants
//! écartés et les structures — ce qui dit où l'analyse s'arrête. Ne suit pas les liens, ni les dossiers de bâti.
//! `cargo test --release --test probe_coverage -- --ignored --nocapture`.

use std::collections::BTreeMap;

use codeparsers::coverage::{measure, Category};

fn fichiers(d: &std::path::Path, v: &mut Vec<std::path::PathBuf>) {
    let Ok(lu) = std::fs::read_dir(d) else { return };
    for e in lu.flatten() {
        let p = e.path();
        let nom = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if nom.starts_with('.') || matches!(nom, "target" | "node_modules" | "build" | "dist") || e.file_type().map(|t| t.is_symlink()).unwrap_or(true) {
            continue;
        }
        if p.is_dir() {
            fichiers(&p, v)
        } else {
            v.push(p)
        }
    }
}

#[test]
#[ignore]
fn the_coverage_of_a_tree() {
    let dirs = std::env::var("SONDE_DIRS").expect("SONDE_DIRS");
    let cats = [Category::Reference, Category::Declaration, Category::Excluded, Category::Neutral, Category::Structure];
    for racine in dirs.split(',') {
        let mut v = Vec::new();
        fichiers(std::path::Path::new(racine), &mut v);
        v.sort();
        // langue (extension) → catégorie → nombre ; et, par catégorie écartée / structure, sorte → nombre.
        let mut par_langue: BTreeMap<String, BTreeMap<Category, usize>> = BTreeMap::new();
        let mut sortes: BTreeMap<(String, Category), BTreeMap<String, usize>> = BTreeMap::new();
        let mut lus = 0;
        for p in &v {
            let rel = p.strip_prefix(racine).unwrap_or(p).to_string_lossy().to_string();
            let Ok(texte) = std::fs::read_to_string(p) else { continue };
            if texte.len() > 1_000_000 {
                continue;
            }
            let Some(c) = measure(&rel, &texte) else { continue };
            lus += 1;
            let langue = rel.rsplit('.').next().unwrap_or("").to_string();
            for (sorte, m) in &c.by_kind {
                for (cat, n) in m {
                    *par_langue.entry(langue.clone()).or_default().entry(*cat).or_default() += n;
                    if matches!(cat, Category::Excluded | Category::Structure) {
                        *sortes.entry((langue.clone(), *cat)).or_default().entry(sorte.clone()).or_default() += n;
                    }
                }
            }
        }
        println!("COUVERTURE {racine} : {lus} fichiers lus");
        for (langue, m) in &par_langue {
            let total: usize = m.values().sum();
            let parts: Vec<String> = cats.iter().map(|c| format!("{c:?} {:.1} %", 100.0 * *m.get(c).unwrap_or(&0) as f64 / total.max(1) as f64)).collect();
            println!("COUVERTURE   .{langue} : {total} nœuds nommés ; {}", parts.join(", "));
            for cat in [Category::Excluded, Category::Structure] {
                if let Some(s) = sortes.get(&(langue.clone(), cat)) {
                    let mut l: Vec<(&String, &usize)> = s.iter().collect();
                    l.sort_by(|a, b| b.1.cmp(a.1));
                    let tete: Vec<String> = l.iter().take(8).map(|(k, n)| format!("{k} {n}")).collect();
                    println!("COUVERTURE     {cat:?} par sorte : {}", tete.join(", "));
                }
            }
        }
    }
}
