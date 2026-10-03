//! Sonde : les relations de l'analyseur selon la taille du paquet — en un
//! appel, ou par lots de 64 lignes de la liste (comme l'index).
//!
//! `cargo run --release --example sonde_relations -- <liste> <racine> [échantillon]`
//! Rend : les relations par type dans chaque forme ; la différence, classée
//! (cible au nom unique dans tout le projet — ce que la voie des
//! rendez-vous poserait aussi —, ou nom à plusieurs définitions — le
//! résolveur a pris le premier homonyme) ; et un échantillon tiré au
//! hasard, avec la ligne du site, pour relecture.

use std::collections::{BTreeMap, HashMap, HashSet};

use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectAnalysis, ProjectParser, ProjectParserOptions};
use codeparsers::relationship_resolution::types::{RelationshipResolverOptions, ResolvedRelationship};

type Cle = (String, String, String, String, String);

fn cle(r: &ResolvedRelationship) -> Cle {
    (format!("{:?}", r.r#type), r.from_file.clone(), r.from_name.clone(), r.to_file.clone(), r.to_name.clone())
}

fn analyser(racine: &str, fichiers: &[String], contenus: &HashMap<String, String>) -> ProjectAnalysis {
    ProjectParser::new(ProjectParserOptions { verbose: false }).parse_project(ParseProjectOptions {
        root: racine.to_string(),
        files: fichiers.to_vec(),
        content_map: Some(fichiers.iter().filter_map(|f| contenus.get(f).map(|c| (f.clone(), c.clone()))).collect()),
        resolve_relationships: Some(true),
        resolver_options: Some(RelationshipResolverOptions {
            include_file_level_refs: Some(false),
            include_child_refs: Some(false),
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
    let contenus: HashMap<String, String> = fichiers.iter().filter_map(|f| std::fs::read_to_string(f).ok().map(|c| (f.clone(), c))).collect();

    let un = analyser(&racine, &fichiers, &contenus);
    let rels_un: Vec<ResolvedRelationship> = un.relationships.as_ref().map(|r| r.relationships.clone()).unwrap_or_default();
    let mut rels_lots: Vec<ResolvedRelationship> = Vec::new();
    for c in lignes.chunks(64) {
        let l: Vec<String> = c.iter().filter(|(code, _)| *code).map(|(_, p)| p.clone()).collect();
        if l.is_empty() {
            continue;
        }
        let r = analyser(&racine, &l, &contenus);
        rels_lots.extend(r.relationships.map(|x| x.relationships).unwrap_or_default());
    }

    // Combien de définitions porte chaque nom, dans tout le projet.
    let mut definitions: HashMap<String, usize> = HashMap::new();
    for fa in un.files.values() {
        for s in &fa.scopes {
            *definitions.entry(s.name.clone()).or_default() += 1;
        }
    }

    let (tu, tl) = (par_type(&rels_un), par_type(&rels_lots));
    println!("relations : un appel {}, lots de 64 {}", rels_un.len(), rels_lots.len());
    println!("{:<20} {:>10} {:>10}", "type", "un appel", "lots");
    for t in tu.keys().chain(tl.keys()).collect::<std::collections::BTreeSet<_>>() {
        println!("{t:<20} {:>10} {:>10}", tu.get(t).unwrap_or(&0), tl.get(t).unwrap_or(&0));
    }

    let k_un: HashSet<Cle> = rels_un.iter().map(cle).collect();
    let k_lots: HashSet<Cle> = rels_lots.iter().map(cle).collect();
    let seulement_un: Vec<&ResolvedRelationship> = rels_un.iter().filter(|r| !k_lots.contains(&cle(r))).collect();
    let seulement_lots: Vec<&ResolvedRelationship> = rels_lots.iter().filter(|r| !k_un.contains(&cle(r))).collect();

    // Classer : même fichier ou non ; cible au nom unique, ou à plusieurs
    // définitions ; et le drapeau du résolveur (`fallback_resolution`).
    let classe = |r: &ResolvedRelationship| -> String {
        let fichier = if r.from_file == r.to_file { "même fichier" } else { "entre fichiers" };
        let n = definitions.get(&r.to_name).copied().unwrap_or(0);
        let nom = if n <= 1 { "nom unique".to_string() } else { format!("nom à {} définitions", if n < 5 { n.to_string() } else { "5+".into() }) };
        let via_import = r.metadata.as_ref().and_then(|m| m.via_import).unwrap_or(false);
        format!("{fichier}, {nom}{}", if via_import { ", par import" } else { "" })
    };
    for (titre, v) in [("seulement en un appel", &seulement_un), ("seulement par lots", &seulement_lots)] {
        let mut c: BTreeMap<String, usize> = BTreeMap::new();
        for r in v.iter() {
            *c.entry(format!("{:?} — {}", r.r#type, classe(r))).or_default() += 1;
        }
        println!("\n{titre} : {}", v.len());
        let mut c: Vec<_> = c.into_iter().collect();
        c.sort_by(|a, b| b.1.cmp(&a.1));
        for (k, n) in c.iter().take(25) {
            println!("  {n:>8}  {k}");
        }
    }

    // Toutes les relations entre fichiers vers un nom à plusieurs définitions,
    // en un appel : ce que le résolveur devine, quelle que soit la forme.
    let devine = rels_un.iter().filter(|r| r.from_file != r.to_file && definitions.get(&r.to_name).copied().unwrap_or(0) > 1).count();
    let entre = rels_un.iter().filter(|r| r.from_file != r.to_file).count();
    println!("\nen un appel : {entre} relations entre fichiers, dont {devine} vers un nom à plusieurs définitions (premier homonyme pris)");

    // L'échantillon : tiré par un hachage fixe, rejouable.
    let mut tirage: Vec<&&ResolvedRelationship> = seulement_un.iter().collect();
    tirage.sort_by_key(|r| {
        let k = cle(r);
        let s = format!("{k:?}");
        s.bytes().fold(1469598103934665603u64, |h, b| (h ^ b as u64).wrapping_mul(1099511628211))
    });
    println!("\néchantillon de {n_ech} (seulement en un appel) :");
    for r in tirage.iter().take(n_ech) {
        let site = r.metadata.as_ref().and_then(|m| m.sites.first().and_then(|s| s.line));
        let texte = site
            .and_then(|l| {
                let abs = std::path::Path::new(&racine).join(&r.from_file);
                let c = contenus.get(&abs.to_string_lossy().to_string())?;
                c.lines().nth(l.saturating_sub(1) as usize).map(|t| t.trim().chars().take(110).collect::<String>())
            })
            .unwrap_or_default();
        println!(
            "- {:?} {}::{} ({}:{}) → {} [{}] ({}) — {} déf. | {}",
            r.r#type,
            r.from_file.rsplit('/').next().unwrap_or(""),
            r.from_name,
            r.from_file,
            site.map(|l| l.to_string()).unwrap_or_default(),
            r.to_name,
            r.to_type,
            r.to_file,
            definitions.get(&r.to_name).copied().unwrap_or(0),
            texte
        );
    }
}
