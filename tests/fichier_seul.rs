//! **Le graphe ne dépend pas du paquet.**
//!
//! Entre fichiers, le résolveur prenait le premier homonyme, sans regarder
//! ni le langage, ni le genre de la cible, ni les imports : sur le dépôt
//! rag3db, 35 relations fausses sur 50 tirées parmi celles qu'il ne pose
//! qu'en voyant tout le projet d'un coup (3 octobre 2026). Et ce qu'il posait
//! dépendait de la taille du paquet qu'on lui donnait : 908 000 relations en
//! un appel, 499 000 par paquets de 64.
//!
//! `resolve_cross_file: Some(false)` : l'analyseur ne résout que dans le
//! fichier ; un lien entre fichiers est l'affaire du consommateur (dans
//! rag3weaver, les rendez-vous par le nom, qui s'abstiennent sur un nom
//! ambigu). `project_files` : les chemins de tout le projet, pour qu'un
//! import vers un module analysé dans un autre paquet ne devienne pas une
//! bibliothèque externe.

use std::collections::{BTreeSet, HashMap};

use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};
use codeparsers::relationship_resolution::types::{RelationshipResolverOptions, RelationshipType, ResolvedRelationship};

const PROJET: &[(&str, &str)] = &[
    ("a.rs", "pub fn helper() -> u32 {\n    1\n}\n\npub struct Node;\n\nimpl Node {\n    pub fn go(&self) -> u32 {\n        local()\n    }\n}\n\nfn local() -> u32 {\n    helper()\n}\n"),
    ("b.rs", "pub fn helper() -> u32 {\n    2\n}\n\npub trait Shape {\n    fn area(&self) -> f64;\n}\n"),
    ("run.rs", "use crate::a::Node;\n\nfn run() -> u32 {\n    let n: Node = Node;\n    helper() + n.go()\n}\n\nstruct Square;\n\nimpl Shape for Square {\n    fn area(&self) -> f64 {\n        1.0\n    }\n}\n"),
    // Le champ est déclaré dans un fichier, lu dans un autre : son type ne
    // doit pas changer la résolution selon que la déclaration est dans le
    // paquet ou non.
    ("store.rs", "pub struct Store {\n    inner: Inner,\n}\n"),
    ("store_impl.rs", "pub struct Inner;\n\nimpl Inner {\n    pub fn get(&self) -> u32 {\n        1\n    }\n}\n\npub struct Other;\n\nimpl Other {\n    pub fn get(&self) -> u32 {\n        2\n    }\n}\n\nimpl Store {\n    pub fn read(&self) -> u32 {\n        self.inner.get()\n    }\n}\n"),
    ("pkg/models.py", "class Base:\n    def save(self):\n        return 1\n\n\ndef make():\n    return Base()\n"),
    ("main.py", "from pkg.models import Base, make\nimport requests\n\n\nclass User(Base):\n    def load(self):\n        requests.get(\"u\")\n        return make()\n"),
];

fn options(fichier_seul: bool) -> RelationshipResolverOptions {
    RelationshipResolverOptions {
        include_file_level_refs: Some(false),
        include_child_refs: Some(false),
        resolve_cross_file: Some(!fichier_seul),
        project_files: fichier_seul.then(|| PROJET.iter().map(|(n, _)| format!("/virtual/{n}")).collect()),
        ..Default::default()
    }
}

fn analyser(noms: &[&str], fichier_seul: bool) -> Vec<ResolvedRelationship> {
    let contenus: HashMap<String, String> = PROJET.iter().filter(|(n, _)| noms.contains(n)).map(|(n, s)| (format!("/virtual/{n}"), s.to_string())).collect();
    ProjectParser::new(ProjectParserOptions { verbose: false })
        .parse_project(ParseProjectOptions {
            root: "/virtual".to_string(),
            files: noms.iter().map(|n| format!("/virtual/{n}")).collect(),
            content_map: Some(contenus),
            resolve_relationships: Some(true),
            resolver_options: Some(options(fichier_seul)),
        })
        .relationships
        .map_or_else(Vec::new, |r| r.relationships)
}

fn cle(r: &ResolvedRelationship) -> String {
    format!("{:?} {}:{} → {}:{}", r.r#type, r.from_file, r.from_name, r.to_file, r.to_name)
}

/// Par paquets de `taille` fichiers, dans l'ordre du projet.
fn par_paquets(taille: usize) -> BTreeSet<String> {
    let noms: Vec<&str> = PROJET.iter().map(|(n, _)| *n).collect();
    noms.chunks(taille).flat_map(|c| analyser(c, true)).map(|r| cle(&r)).collect()
}

#[test]
fn le_graphe_ne_depend_pas_du_paquet() {
    let entier = par_paquets(PROJET.len());
    assert!(!entier.is_empty());
    for taille in [1, 2, 3] {
        let p = par_paquets(taille);
        let manque: Vec<_> = entier.difference(&p).collect();
        let surplus: Vec<_> = p.difference(&entier).collect();
        assert!(manque.is_empty() && surplus.is_empty(), "paquets de {taille} : manque {manque:#?}, surplus {surplus:#?}");
    }
}

#[test]
fn fichier_seul_aucune_relation_ne_traverse_un_fichier() {
    let noms: Vec<&str> = PROJET.iter().map(|(n, _)| *n).collect();
    let rels = analyser(&noms, true);
    let traversent: Vec<String> = rels
        .iter()
        .filter(|r| r.r#type != RelationshipType::USESLIBRARY && r.from_file != r.to_file)
        .map(cle)
        .collect();
    assert!(traversent.is_empty(), "{traversent:#?}");
}

/// Dans un fichier, rien ne change : les relations du mode « fichier seul »
/// sont exactement celles d'aujourd'hui dont les deux bouts sont dans le
/// même fichier.
#[test]
fn fichier_seul_garde_tout_ce_qui_reste_dans_le_fichier() {
    let noms: Vec<&str> = PROJET.iter().map(|(n, _)| *n).collect();
    let seul: BTreeSet<String> = analyser(&noms, true).iter().filter(|r| r.r#type != RelationshipType::USESLIBRARY).map(cle).collect();
    let avant: BTreeSet<String> = analyser(&noms, false)
        .iter()
        .filter(|r| r.r#type != RelationshipType::USESLIBRARY && r.from_file == r.to_file)
        .map(cle)
        .collect();
    // Ce que « fichier seul » ne pose plus dans un fichier : seulement ce
    // qu'une déclaration d'un autre fichier décidait (le type du champ
    // `inner`, déclaré dans store.rs).
    assert!(seul.is_subset(&avant), "{:#?}", seul.difference(&avant).collect::<Vec<_>>());
    let perdu: Vec<&String> = avant.difference(&seul).collect();
    assert!(perdu.iter().all(|k| k.contains("store_impl.rs:") && k.contains(":get")), "{perdu:#?}");
    // Le même fichier se résout toujours : `local` appelle `helper` de a.rs,
    // pas celui de b.rs.
    assert!(seul.iter().any(|k| k.starts_with("CONSUMES a.rs:local → a.rs:helper")), "{seul:#?}");
}

/// Un import vers un module du projet n'est pas une bibliothèque, même quand
/// ce module est analysé dans un autre paquet ; un vrai paquet externe l'est.
#[test]
fn un_module_du_projet_n_est_pas_une_bibliotheque_d_un_autre_paquet() {
    let libs: Vec<String> = analyser(&["main.py"], true)
        .iter()
        .filter(|r| r.r#type == RelationshipType::USESLIBRARY)
        .map(|r| r.to_name.clone())
        .collect();
    assert!(libs.iter().any(|l| l == "requests"), "{libs:?}");
    assert!(!libs.iter().any(|l| l.starts_with("pkg")), "pkg.models est dans le projet : {libs:?}");
}

/// Un dossier du projet qui s'appelle `std`, `string` ou `node` ne fait pas
/// de la bibliothèque du même nom un module du projet : en Rust, `crate`,
/// `self` et `super` disent seuls ce qui est local, en C et C++ la forme de
/// l'`#include`. La liste des modules ne sert qu'à Python, où un import
/// absolu peut viser le projet.
#[test]
fn un_dossier_du_projet_ne_cache_pas_une_bibliotheque() {
    let source = "use std::fmt;\n\npub fn f(x: &dyn fmt::Debug) -> String {\n    format!(\"{x:?}\")\n}\n";
    let contenus = HashMap::from([("/virtual/lib.rs".to_string(), source.to_string())]);
    let libs: Vec<String> = ProjectParser::new(ProjectParserOptions { verbose: false })
        .parse_project(ParseProjectOptions {
            root: "/virtual".to_string(),
            files: vec!["/virtual/lib.rs".to_string()],
            content_map: Some(contenus),
            resolve_relationships: Some(true),
            resolver_options: Some(RelationshipResolverOptions {
                include_file_level_refs: Some(false),
                include_child_refs: Some(false),
                resolve_cross_file: Some(false),
                project_files: Some(vec!["/virtual/lib.rs".into(), "/virtual/vendor/std/mod.py".into()]),
                ..Default::default()
            }),
        })
        .relationships
        .map_or_else(Vec::new, |r| r.relationships)
        .into_iter()
        .filter(|r| r.r#type == RelationshipType::USESLIBRARY)
        .map(|r| r.to_name)
        .collect();
    assert!(libs.iter().any(|l| l == "std"), "{libs:?}");
}

/// Un accès de champ — pas un appel — par `self` ou par une variable sans
/// type lu ne vise pas une méthode homonyme du fichier : `p.name` dans
/// `|p| p.name == name` n'utilise pas `GraphTool::name`. La même règle que
/// les rendez-vous de rag3weaver ; un appel garde sa résolution.
#[test]
fn un_acces_de_champ_ne_vise_pas_une_methode_homonyme() {
    let source = "pub struct Tool {\n    label: String,\n}\n\nimpl Tool {\n    pub fn name(&self) -> &str {\n        &self.label\n    }\n    pub fn label(&self) -> usize {\n        self.label.len()\n    }\n}\n\nstruct P {\n    name: String,\n}\n\nfn find(ps: &[P], name: &str) -> bool {\n    ps.iter().any(|p| p.name == name)\n}\n\nfn appelle(t: &Tool, u: &Tool) -> usize {\n    t.name().len() + u.name().len()\n}\n";
    let contenus = HashMap::from([("/virtual/lib.rs".to_string(), source.to_string())]);
    let rels: Vec<String> = ProjectParser::new(ProjectParserOptions { verbose: false })
        .parse_project(ParseProjectOptions {
            root: "/virtual".to_string(),
            files: vec!["/virtual/lib.rs".to_string()],
            content_map: Some(contenus),
            resolve_relationships: Some(true),
            resolver_options: Some(options(true)),
        })
        .relationships
        .map_or_else(Vec::new, |r| r.relationships)
        .iter()
        .filter(|r| r.r#type == RelationshipType::CONSUMES)
        .map(cle)
        .collect();
    assert!(!rels.iter().any(|k| k.contains(":find → ") && k.ends_with(":name")), "p.name est un champ : {rels:#?}");
    assert!(!rels.iter().any(|k| k.contains(":Closure → ") && k.ends_with(":name")), "p.name est un champ : {rels:#?}");
    assert!(!rels.iter().any(|k| k.contains(":name → ") && k.ends_with(":label")), "self.label est un champ : {rels:#?}");
    assert!(rels.iter().any(|k| k.contains(":appelle → ") && k.ends_with(":name")), "t.name() est un appel : {rels:#?}");
}
