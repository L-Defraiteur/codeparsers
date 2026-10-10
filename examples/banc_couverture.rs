//! **Le banc de couverture d'un dépôt** — ce que codeparsers comprend, et ce
//! qu'il ne sait pas comprendre, mesuré sur nos dépôts.
//!
//! Pour un dépôt (une ou plusieurs racines) : les fichiers par langage et la
//! part de leurs octets que tree-sitter n'a pas su lire ; les scopes par
//! genre ; les références par genre d'usage — combien deviennent une
//! relation, en fichier seul (le mode de rag3weaver) et entre fichiers, et
//! pourquoi les autres ne le deviennent pas (le relevé des non-résolues).
//!
//! Une ligne de référence (`clé = valeur`, triée) se commite dans `banc/` ;
//! `--verifier` la recalcule et échoue si une mesure a reculé dans le
//! mauvais sens (une part reliée qui baisse, des erreurs qui montent). Les
//! comptes neutres (nombre de scopes, de fichiers) changent sans échouer :
//! ils se relisent.
//!
//! Usage :
//!   cargo run --release --example banc_couverture -- <nom> <racine>... [--ecrire banc/<nom>.txt | --verifier banc/<nom>.txt]

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

use codeparsers::parallel::parser_worker::SupportedLanguage;
use codeparsers::parallel::project_parser::{detect_language_from_path, ParseProjectOptions, ProjectParser, ProjectParserOptions};
use codeparsers::relationship_resolution::types::RelationshipResolverOptions;
use codeparsers::scope_extraction::types::{IdentifierReferenceKind, ScopeInfo};

const IGNORES: &[&str] = &["target", ".git", "node_modules", "dist", "build", "generated", ".venv", "third_party", "vendor"];

fn walk(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if IGNORES.contains(&name.as_str()) {
            continue;
        }
        let Ok(genre) = entry.file_type() else { continue };
        if genre.is_symlink() {
            continue;
        }
        if genre.is_dir() {
            walk(&entry.path(), out);
        } else {
            out.push(entry.path());
        }
    }
}

fn grammaire(lang: &SupportedLanguage) -> tree_sitter::Language {
    match lang {
        SupportedLanguage::Typescript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        SupportedLanguage::Javascript => tree_sitter_typescript::LANGUAGE_TSX.into(),
        SupportedLanguage::Python => tree_sitter_python::LANGUAGE.into(),
        SupportedLanguage::Rust => tree_sitter_rust::LANGUAGE.into(),
        SupportedLanguage::Go => tree_sitter_go::LANGUAGE.into(),
        SupportedLanguage::C => tree_sitter_c::LANGUAGE.into(),
        SupportedLanguage::Cpp => tree_sitter_cpp::LANGUAGE.into(),
        SupportedLanguage::Csharp => tree_sitter_c_sharp::LANGUAGE.into(),
    }
}

/// Les octets sous un nœud ERROR ou manquant.
fn octets_en_erreur(lang: &SupportedLanguage, contenu: &str) -> usize {
    let mut p = tree_sitter::Parser::new();
    if p.set_language(&grammaire(lang)).is_err() {
        return 0;
    }
    let Some(arbre) = p.parse(contenu, None) else { return contenu.len() };
    let mut octets = 0;
    let mut pile = vec![arbre.root_node()];
    while let Some(n) = pile.pop() {
        if n.kind() == "ERROR" || n.is_missing() {
            octets += n.end_byte() - n.start_byte();
            continue;
        }
        let mut c = n.walk();
        for e in n.children(&mut c) {
            pile.push(e);
        }
    }
    octets
}

fn tous<'a>(s: &'a ScopeInfo, out: &mut Vec<&'a ScopeInfo>) {
    out.push(s);
    for c in &s.children {
        tous(c, out);
    }
}

fn langue(l: &SupportedLanguage) -> &'static str {
    match l {
        SupportedLanguage::Typescript => "typescript",
        SupportedLanguage::Javascript => "javascript",
        SupportedLanguage::Python => "python",
        SupportedLanguage::Rust => "rust",
        SupportedLanguage::Go => "go",
        SupportedLanguage::C => "c",
        SupportedLanguage::Cpp => "cpp",
        SupportedLanguage::Csharp => "csharp",
    }
}

fn pct(a: usize, b: usize) -> String {
    if b == 0 { "0.0".into() } else { format!("{:.1}", 100.0 * a as f64 / b as f64) }
}

/// Le bon sens d'une mesure : +1 elle doit monter, -1 descendre, 0 neutre.
fn sens(cle: &str) -> i32 {
    if cle.ends_with(".relies_pct") || cle.contains(".appelants.") || cle.starts_with("verrous.") {
        1
    } else if cle.ends_with(".erreur_pct") || cle.contains(".non_resolues") {
        -1
    } else {
        0
    }
}

/// La forme d'une référence non résolue, lue sur la référence elle-même.
fn forme(nom: &str, usage: &str, qualif: Option<&str>) -> &'static str {
    let majuscules = nom.len() > 1 && nom.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_');
    let racine = qualif.map(|q| q.split([':', '.', '-']).next().unwrap_or(""));
    if racine == Some("std") {
        return "std:: (externe)";
    }
    if majuscules {
        return "macro ou constante (MAJUSCULES)";
    }
    match (usage, qualif) {
        ("lock" | "sharedlock", _) => "verrou sur un champ (au consommateur)",
        ("import", _) => "import",
        ("type", _) => "type",
        ("inheritance", _) => "héritage",
        ("call", None) => "appel nu",
        ("call", Some(q)) if q.contains("::") && !q.contains('.') && !q.contains("->") => "appel par chemin (Ns::f, T::f)",
        ("call", Some(_)) => "méthode sur un receveur (x.f(), x->f())",
        (_, Some(q)) if q.contains("::") && !q.contains('.') && !q.contains("->") => "membre par chemin (T::x)",
        (_, Some(_)) => "lecture de champ (x.y, x->y)",
        _ => "nom seul",
    }
}

/// Ce qu'une forme coûte aux outils (usages, impact, voisinage), et le lot
/// qui la fermerait.
fn cout(forme: &str) -> (f64, &'static str) {
    match forme {
        "méthode sur un receveur (x.f(), x->f())" => (3.0, "B2 receveurs typés"),
        "appel par chemin (Ns::f, T::f)" => (3.0, "B1 chemins / rendez-vous"),
        "appel nu" => (3.0, "B1 rendez-vous (autre fichier) / locaux"),
        "héritage" => (2.0, "B1"),
        "type" => (1.5, "B1"),
        "import" => (1.0, "#include → fichier"),
        "membre par chemin (T::x)" => (1.0, "membres statiques, enums"),
        "lecture de champ (x.y, x->y)" => (0.5, "champs : ne pas relier (comme Rust)"),
        "macro ou constante (MAJUSCULES)" => (0.5, "macros #define"),
        "nom seul" => (0.5, "paramètres et locaux"),
        _ => (0.0, "externe : à dire compris"),
    }
}

fn rapport_des_trous(
    mode: &str,
    non_resolues: &[codeparsers::relationship_resolution::types::UnresolvedReference],
    refs: &HashMap<(String, String, String), String>,
    qualifs: &HashMap<(String, String, String), Option<String>>,
) {
    // Un trou dans le stockage ou les transactions pèse double : c'est là
    // qu'on cherche les verrous.
    let critique = |f: &str| f.starts_with("storage/") || f.starts_with("transaction/") || f.contains("/storage/") || f.contains("/transaction/");
    let mut lignes: BTreeMap<&'static str, (usize, usize, f64, Vec<String>)> = BTreeMap::new();
    for u in non_resolues {
        let cle = (u.from_file.trim_start_matches('/').to_string(), u.from_scope.clone(), u.identifier.clone());
        let usage = refs.get(&cle).map_or(u.kind.as_str(), |x| x.as_str());
        let q = qualifs.get(&cle).cloned().flatten();
        let f = forme(&u.identifier, usage, q.as_deref());
        let (poids, _) = cout(f);
        let e = lignes.entry(f).or_default();
        e.0 += 1;
        let c = critique(&cle.0);
        if c {
            e.1 += 1;
        }
        e.2 += poids * if c { 2.0 } else { 1.0 };
        if e.3.len() < 3 && u.reason.starts_with("aucun") || e.3.len() < 3 && c {
            e.3.push(format!("`{}{}` ({})", q.map(|q| format!("{q}.")).unwrap_or_default(), u.identifier, cle.0.rsplit('/').next().unwrap_or("")));
        }
    }
    let mut tri: Vec<_> = lignes.into_iter().collect();
    tri.sort_by(|a, b| b.1 .2.partial_cmp(&a.1 .2).unwrap());
    println!("\n## Les trous ({mode}), classés par coût pour les outils\n");
    println!("| forme | non résolues | dont storage/transaction | coût | lot qui la ferme | exemples |\n|---|---|---|---|---|---|");
    for (f, (n, c, poids, ex)) in &tri {
        println!("| {f} | {n} | {c} | {poids:.0} | {} | {} |", cout(f).1, ex.join(", "));
    }
}

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let mut ecrire = None;
    let mut verifier = None;
    let trous = args.iter().any(|a| a == "--trous");
    args.retain(|a| a != "--trous");
    let mut appelants_de = None;
    if let Some(i) = args.iter().position(|a| a == "--appelants") {
        appelants_de = args.get(i + 1).cloned();
        args.drain(i..(i + 2).min(args.len()));
    }
    if let Some(i) = args.iter().position(|a| a == "--ecrire") {
        ecrire = args.get(i + 1).cloned();
        args.drain(i..(i + 2).min(args.len()));
    }
    if let Some(i) = args.iter().position(|a| a == "--verifier") {
        verifier = args.get(i + 1).cloned();
        args.drain(i..(i + 2).min(args.len()));
    }
    if args.len() < 2 {
        eprintln!("usage: banc_couverture <nom> <racine>... [--ecrire f | --verifier f]");
        std::process::exit(2);
    }
    let nom = args.remove(0);
    let racines = args;

    let mut chemins = Vec::new();
    for r in &racines {
        walk(Path::new(r), &mut chemins);
    }
    chemins.sort();
    let mut m: BTreeMap<String, String> = BTreeMap::new();
    let mut contenus = HashMap::new();
    let mut files = Vec::new();
    let mut par_langue: BTreeMap<&str, (usize, usize, usize)> = BTreeMap::new();
    for c in &chemins {
        let s = c.to_string_lossy().to_string();
        let Some(l) = detect_language_from_path(&s) else { continue };
        let Ok(texte) = std::fs::read_to_string(c) else { continue };
        let e = par_langue.entry(langue(&l)).or_default();
        e.0 += 1;
        e.1 += texte.len();
        e.2 += octets_en_erreur(&l, &texte);
        contenus.insert(s.clone(), texte);
        files.push(s);
    }
    for (l, (n, o, e)) in &par_langue {
        m.insert(format!("fichiers.{l}"), n.to_string());
        m.insert(format!("octets.{l}.ko"), (o / 1024).to_string());
        m.insert(format!("analyse.{l}.erreur_pct"), pct(*e, *o));
    }
    // Une racine commune pour les chemins relatifs.
    let racine = racines[0].trim_end_matches('/').to_string();
    let racine_commune = if racines.len() == 1 { racine.clone() } else { Path::new(&racine).parent().map_or(racine.clone(), |p| p.to_string_lossy().to_string()) };

    let mut pires: Vec<(usize, String)> = Vec::new();
    for croise in [false, true] {
        let mode = if croise { "entre_fichiers" } else { "fichier_seul" };
        let r = ProjectParser::new(ProjectParserOptions { verbose: false }).parse_project(ParseProjectOptions {
            root: racine_commune.clone(),
            files: files.clone(),
            content_map: Some(contenus.clone()),
            resolve_relationships: Some(true),
            resolver_options: Some(RelationshipResolverOptions {
                include_file_level_refs: Some(false),
                include_child_refs: Some(false),
                resolve_cross_file: Some(croise),
                ..Default::default()
            }),
        });
        let mut scopes: Vec<&ScopeInfo> = Vec::new();
        for fa in r.files.values() {
            for s in &fa.scopes {
                tous(s, &mut scopes);
            }
        }
        // Les références relevées, une par (fichier, scope, nom), par genre d'usage.
        let mut refs: HashMap<(String, String, String), String> = HashMap::new();
        let mut qualifs: HashMap<(String, String, String), Option<String>> = HashMap::new();
        for s in &scopes {
            for x in &s.identifier_references {
                // Un builtin est compris sans relation ; un nom du fichier
                // (`LocalScope`) se relie dans son fichier : il compte, relié.
                if matches!(x.kind, Some(IdentifierReferenceKind::Builtin)) || x.identifier.is_empty() || x.identifier == s.name {
                    continue;
                }
                let usage = x.usage.as_ref().map_or_else(|| "other".to_string(), |u| format!("{u:?}").to_lowercase());
                let f = s.file_path.strip_prefix(&racine_commune).unwrap_or(&s.file_path).trim_start_matches('/').to_string();
                qualifs.entry((f.clone(), s.name.clone(), x.identifier.clone())).or_insert_with(|| x.qualifier.clone());
                refs.entry((f, s.name.clone(), x.identifier.clone())).or_insert(usage);
            }
        }
        let Some(rel) = r.relationships.as_ref() else { continue };
        let non_resolues: HashSet<(String, String, String)> = rel
            .unresolved_references
            .iter()
            .map(|u| (u.from_file.trim_start_matches('/').to_string(), u.from_scope.clone(), u.identifier.clone()))
            .collect();
        let mut par_usage: BTreeMap<String, (usize, usize)> = BTreeMap::new();
        for (cle, usage) in &refs {
            let e = par_usage.entry(usage.clone()).or_default();
            e.0 += 1;
            if !non_resolues.contains(cle) {
                e.1 += 1;
            }
        }
        for (u, (n, relies)) in &par_usage {
            m.insert(format!("{mode}.refs.{u}"), n.to_string());
            m.insert(format!("{mode}.refs.{u}.relies_pct"), pct(*relies, *n));
        }
        let mut par_raison: BTreeMap<String, usize> = BTreeMap::new();
        let mut par_genre: BTreeMap<String, usize> = BTreeMap::new();
        let mut par_fichier: BTreeMap<String, usize> = BTreeMap::new();
        for u in &rel.unresolved_references {
            *par_raison.entry(u.reason.split(" (").next().unwrap_or(&u.reason).replace(' ', "_")).or_default() += 1;
            *par_genre.entry(u.from_type.clone()).or_default() += 1;
            if u.kind == "call" {
                *par_fichier.entry(u.from_file.clone()).or_default() += 1;
            }
        }
        m.insert(format!("{mode}.non_resolues"), rel.unresolved_references.len().to_string());
        if !croise {
            let (mut exclusifs, mut partages) = (0usize, 0usize);
            for s in &scopes {
                // Par le nom du genre : le banc se compile aussi contre un
                // codeparsers d'avant (banc/comparer.sh), qui n'a pas ces genres.
                for x in &s.identifier_references {
                    match x.usage.as_ref().map(|u| format!("{u:?}")).as_deref() {
                        Some("Lock") => exclusifs += 1,
                        Some("SharedLock") => partages += 1,
                        _ => {}
                    }
                }
            }
            m.insert("verrous.exclusifs".into(), exclusifs.to_string());
            m.insert("verrous.partages".into(), partages.to_string());
        }
        if trous {
            rapport_des_trous(mode, &rel.unresolved_references, &refs, &qualifs);
        }
        // Les appelants d'une méthode (`Classe::methode`) : les CONSUMES vers
        // un scope de ce nom dont le parent est cette classe.
        if let Some(cible) = &appelants_de {
            let (classe, methode) = cible.rsplit_once("::").unwrap_or(("", cible.as_str()));
            let cibles: HashSet<&str> = scopes
                .iter()
                .filter(|s| s.name == methode && (classe.is_empty() || s.parent.as_deref().is_some_and(|p| p.rsplit("::").next() == Some(classe))))
                .map(|s| s.file_path.as_str())
                .collect();
            let appelants: Vec<String> = rel
                .relationships
                .iter()
                .filter(|x| x.r#type == codeparsers::relationship_resolution::types::RelationshipType::CONSUMES && x.to_name == methode)
                .filter(|x| cibles.iter().any(|f| f.ends_with(x.to_file.trim_start_matches('/'))))
                .map(|x| format!("{}:{}", x.from_file.rsplit('/').next().unwrap_or(""), x.from_name))
                .collect();
            m.insert(format!("{mode}.appelants.{}", cible.replace("::", "__")), appelants.len().to_string());
            println!("# appelants de {cible} ({mode}) : {} {:?}", appelants.len(), appelants);
        }
        for (k, n) in &par_raison {
            m.insert(format!("{mode}.non_resolues.raison.{k}"), n.to_string());
        }
        for (k, n) in &par_genre {
            m.insert(format!("{mode}.non_resolues.genre.{k}"), n.to_string());
        }
        if !croise {
            let mut genres: BTreeMap<String, usize> = BTreeMap::new();
            for s in &scopes {
                *genres.entry(format!("{:?}", s.r#type)).or_default() += 1;
            }
            for (g, n) in &genres {
                m.insert(format!("scopes.{g}"), n.to_string());
            }
            m.insert("scopes.total".into(), scopes.len().to_string());
        } else {
            pires = par_fichier.into_iter().map(|(f, n)| (n, f)).collect();
            pires.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        }
    }

    let texte: String = m.iter().map(|(k, v)| format!("{k} = {v}\n")).collect();
    println!("# banc de couverture — {nom}\n{texte}");
    println!("# appels non résolus par fichier (entre fichiers), les vingt premiers");
    for (n, f) in pires.iter().take(20) {
        println!("#   {n:>5}  {f}");
    }
    if let Some(f) = ecrire {
        std::fs::write(&f, format!("# banc de couverture — {nom}\n{texte}")).expect("écrire la référence");
        println!("référence écrite : {f}");
    }
    if let Some(f) = verifier {
        let reference: BTreeMap<String, String> = std::fs::read_to_string(&f)
            .expect("lire la référence")
            .lines()
            .filter(|l| !l.starts_with('#'))
            .filter_map(|l| l.split_once(" = ").map(|(k, v)| (k.to_string(), v.to_string())))
            .collect();
        let mut recul = Vec::new();
        for (k, ancien) in &reference {
            let Some(nouveau) = m.get(k) else {
                println!("disparue : {k} (était {ancien})");
                continue;
            };
            if nouveau == ancien {
                continue;
            }
            let (a, b) = (ancien.parse::<f64>().unwrap_or(0.0), nouveau.parse::<f64>().unwrap_or(0.0));
            let s = sens(k);
            let mauvais = (s > 0 && b < a) || (s < 0 && b > a);
            println!("{} {k} : {ancien} → {nouveau}", if mauvais { "RECUL" } else if s == 0 { "change" } else { "monte" });
            if mauvais {
                recul.push(k.clone());
            }
        }
        for k in m.keys().filter(|k| !reference.contains_key(*k)) {
            println!("nouvelle : {k} = {}", m[k]);
        }
        if !recul.is_empty() {
            eprintln!("{} mesure(s) ont reculé", recul.len());
            std::process::exit(1);
        }
        println!("aucune mesure n'a reculé");
    }
}
