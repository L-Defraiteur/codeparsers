//! Sonde : « fichier seul » sur une liste de `sonde_analyse`.
//!
//! `cargo run --release --example sonde_fichier_seul -- <liste> <racine> [échantillon]`
//! A : par lots de 64 lignes, mode d'aujourd'hui (ce que l'index contient) ;
//! B : par lots de 64, fichier seul (`project_files` = toute la liste) ;
//! C : un appel, fichier seul. Rend les comptes par type, B == C relation à
//! relation, ce que A pose et que B retire (classé, avec la part qu'un
//! import du fichier appelant désigne), un échantillon tiré au hasard de ces
//! retraits, et les durées de résolution.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::time::Instant;

use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectAnalysis, ProjectParser, ProjectParserOptions};
use codeparsers::relationship_resolution::types::{RelationshipResolverOptions, RelationshipType, ResolvedRelationship};

type Cle = (String, String, String, String, String);

fn cle(r: &ResolvedRelationship) -> Cle {
    (format!("{:?}", r.r#type), r.from_file.clone(), r.from_name.clone(), r.to_file.clone(), r.to_name.clone())
}

fn analyser(racine: &str, fichiers: &[String], contenus: &HashMap<String, String>, projet: Option<&Vec<String>>) -> ProjectAnalysis {
    ProjectParser::new(ProjectParserOptions { verbose: false }).parse_project(ParseProjectOptions {
        root: racine.to_string(),
        files: fichiers.to_vec(),
        content_map: Some(fichiers.iter().filter_map(|f| contenus.get(f).map(|c| (f.clone(), c.clone()))).collect()),
        resolve_relationships: Some(true),
        resolver_options: Some(RelationshipResolverOptions {
            include_file_level_refs: Some(false),
            include_child_refs: Some(false),
            resolve_cross_file: Some(projet.is_none()),
            project_files: projet.cloned(),
            ..Default::default()
        }),
    })
}

fn par_type(rels: &[ResolvedRelationship]) -> BTreeMap<String, usize> {
    let mut m = BTreeMap::new();
    for r in rels {
        *m.entry(format!("{:?}", r.r#type)).or_default() += 1;
    }
    m
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let liste = std::fs::read_to_string(&args[1]).expect("liste");
    let racine = args[2].clone();
    let n_ech: usize = args.get(3).and_then(|v| v.parse().ok()).unwrap_or(50);
    let lignes: Vec<(bool, String)> = liste
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| match l.split_once('\t') {
            Some((m, p)) => (m == "C", p.to_string()),
            None => (true, l.to_string()),
        })
        .collect();
    let fichiers: Vec<String> = lignes.iter().filter(|(c, _)| *c).map(|(_, p)| p.clone()).collect();
    let projet: Vec<String> = lignes.iter().map(|(_, p)| p.clone()).collect();
    let contenus: HashMap<String, String> = fichiers.iter().filter_map(|f| std::fs::read_to_string(f).ok().map(|c| (f.clone(), c))).collect();

    let par_lots = |projet: Option<&Vec<String>>| -> (Vec<ResolvedRelationship>, u128, Vec<ProjectAnalysis>) {
        let mut rels = Vec::new();
        let mut ms = 0;
        let mut analyses = Vec::new();
        for c in lignes.chunks(64) {
            let l: Vec<String> = c.iter().filter(|(code, _)| *code).map(|(_, p)| p.clone()).collect();
            if l.is_empty() {
                continue;
            }
            let mut a = analyser(&racine, &l, &contenus, projet);
            ms += a.stats.relationship_time_ms.unwrap_or(0);
            rels.extend(a.relationships.take().map(|x| x.relationships).unwrap_or_default());
            analyses.push(a);
        }
        (rels, ms, analyses)
    };
    let (a, ms_a, analyses_a) = par_lots(None);
    let (b, ms_b, _) = par_lots(Some(&projet));
    let t = Instant::now();
    let c_an = analyser(&racine, &fichiers, &contenus, Some(&projet));
    let c_total = t.elapsed().as_millis();
    let ms_c = c_an.stats.relationship_time_ms.unwrap_or(0);
    let c: Vec<ResolvedRelationship> = c_an.relationships.as_ref().map(|r| r.relationships.clone()).unwrap_or_default();

    let (ta, tb, tc) = (par_type(&a), par_type(&b), par_type(&c));
    println!("relations : A (lots, aujourd'hui) {}, B (lots, fichier seul) {}, C (un appel, fichier seul) {}", a.len(), b.len(), c.len());
    println!("{:<14} {:>9} {:>9} {:>9}", "type", "A", "B", "C");
    for t in ta.keys().chain(tb.keys()).chain(tc.keys()).collect::<std::collections::BTreeSet<_>>() {
        println!("{t:<14} {:>9} {:>9} {:>9}", ta.get(t).unwrap_or(&0), tb.get(t).unwrap_or(&0), tc.get(t).unwrap_or(&0));
    }
    let kb: HashSet<Cle> = b.iter().map(cle).collect();
    let kc: HashSet<Cle> = c.iter().map(cle).collect();
    println!("B == C : {} (B seul {}, C seul {})", kb == kc, kb.difference(&kc).count(), kc.difference(&kb).count());
    for k in kb.symmetric_difference(&kc).take(10) {
        println!("  écart : {k:?}");
    }
    println!("résolution : A {ms_a} ms (85 lots), B {ms_b} ms (85 lots), C {ms_c} ms (un appel ; appel entier {c_total} ms)");

    // Ce que A pose et que B retire.
    let retires: Vec<&ResolvedRelationship> = a.iter().filter(|r| !kb.contains(&cle(r)) && r.r#type != RelationshipType::CONSUMEDBY).collect();
    let mut definitions: HashMap<String, usize> = HashMap::new();
    for fa in c_an.files.values() {
        for s in &fa.scopes {
            *definitions.entry(s.name.clone()).or_default() += 1;
        }
    }
    // L'origine d'import d'une référence au nom `to_name` dans le scope source.
    let mut origines: HashMap<(String, String), String> = HashMap::new();
    for an in &analyses_a {
        for (f, fa) in &an.files {
            let rel = f.strip_prefix(&format!("{racine}/")).unwrap_or(f).to_string();
            for s in &fa.scopes {
                for r in &s.identifier_references {
                    if let Some(o) = &r.import_origin {
                        origines.insert((format!("{rel}:{}", s.name), r.identifier.clone()), o.source.clone());
                    }
                }
            }
        }
    }
    let mut classes: BTreeMap<String, usize> = BTreeMap::new();
    for r in &retires {
        let n = definitions.get(&r.to_name).copied().unwrap_or(0);
        let nom = if n <= 1 { "nom unique" } else { "nom à plusieurs définitions" };
        let imp = if origines.contains_key(&(format!("{}:{}", r.from_file, r.from_name), r.to_name.clone())) { ", un import le désigne" } else { "" };
        *classes.entry(format!("{:?} — {}{}", r.r#type, nom, imp)).or_default() += 1;
    }
    println!("\nretirées (A sans B, inverses non comptées) : {}", retires.len());
    let mut v: Vec<_> = classes.into_iter().collect();
    v.sort_by(|x, y| y.1.cmp(&x.1));
    for (k, n) in v {
        println!("  {n:>8}  {k}");
    }

    let mut tirage = retires.clone();
    tirage.sort_by_key(|r| format!("{:?}", cle(r)).bytes().fold(1469598103934665603u64, |h, b| (h ^ b as u64).wrapping_mul(1099511628211)));
    println!("\néchantillon de {n_ech} retirées :");
    for r in tirage.iter().take(n_ech) {
        let site = r.metadata.as_ref().and_then(|m| m.sites.first().and_then(|s| s.line));
        let texte = site
            .and_then(|l| {
                let abs = std::path::Path::new(&racine).join(&r.from_file).to_string_lossy().to_string();
                contenus.get(&abs)?.lines().nth(l.saturating_sub(1)).map(|t| t.trim().chars().take(110).collect::<String>())
            })
            .unwrap_or_default();
        let imp = origines.get(&(format!("{}:{}", r.from_file, r.from_name), r.to_name.clone())).cloned().unwrap_or_default();
        println!(
            "- {:?} {} ({}:{}) → {} [{}] ({}) — {} déf.{} | {}",
            r.r#type,
            r.from_name,
            r.from_file,
            site.map(|l| l.to_string()).unwrap_or_default(),
            r.to_name,
            r.to_type,
            r.to_file,
            definitions.get(&r.to_name).copied().unwrap_or(0),
            if imp.is_empty() { String::new() } else { format!(", import {imp}") },
            texte
        );
    }
}
