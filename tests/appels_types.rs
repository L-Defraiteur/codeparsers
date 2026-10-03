//! **Un appel de méthode sur une variable dont le type se lit devient une
//! relation vers la méthode de ce type.**
//!
//! `node.with_delai(t)` ne devenait jamais une relation : le qualificatif
//! `node` n'est pas un scope. Le type d'une variable se lit ici, sans
//! inférence, dans (a) son annotation, (b) un paramètre typé, (c) un
//! initialiseur constructeur (`Type::new(…)`, littéral de struct,
//! `new Type()`, `Type()` en Python). Quand il ne se lit pas, pas de relation,
//! comme avant.
//!
//! Dans chaque cas, le type concurrent (`Other`, qui a une méthode du même
//! nom) est déclaré **avant** le bon : un test ne peut pas passer parce que
//! le résolveur aurait pris le premier homonyme.

use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};
use codeparsers::relationship_resolution::types::{RelationshipResolverOptions, RelationshipType};
use std::collections::HashMap;

/// (source, cible, parent de la cible) de chaque CONSUMES.
fn appels(fichiers: &[(&str, &str)]) -> Vec<(String, String, Option<String>)> {
    let mut contenus = HashMap::new();
    let mut chemins = Vec::new();
    for (nom, source) in fichiers {
        let chemin = format!("/virtual/{nom}");
        contenus.insert(chemin.clone(), source.to_string());
        chemins.push(chemin);
    }
    let res = ProjectParser::new(ProjectParserOptions { verbose: false })
        .parse_project(ParseProjectOptions {
            root: "/virtual".to_string(),
            files: chemins,
            content_map: Some(contenus),
            resolve_relationships: Some(true),
            resolver_options: Some(RelationshipResolverOptions {
                include_file_level_refs: Some(false),
                include_child_refs: Some(false),
                ..Default::default()
            }),
        })
        .relationships
        .expect("des relations");
    res.relationships
        .iter()
        .filter(|r| r.r#type == RelationshipType::CONSUMES)
        .map(|r| (r.from_name.clone(), r.to_name.clone(), res.uuid_mapping.get(&r.to_uuid).and_then(|e| e.parent.clone())))
        .collect()
}

fn appelle(rels: &[(String, String, Option<String>)], de: &str, methode: &str, type_: &str) {
    let cibles: Vec<_> = rels.iter().filter(|(f, t, _)| f == de && t == methode).collect();
    assert!(
        cibles.len() == 1 && cibles[0].2.as_deref() == Some(type_),
        "{de} devrait appeler {type_}::{methode}, et lui seul ; trouvé : {cibles:?}"
    );
}

fn n_appelle_pas(rels: &[(String, String, Option<String>)], de: &str, methode: &str) {
    let cibles: Vec<_> = rels.iter().filter(|(f, t, _)| f == de && t == methode).collect();
    assert!(cibles.is_empty(), "{de} → {methode} ne se lit pas sans inférence ; trouvé : {cibles:?}");
}

// ---------------------------------------------------------------------------
// Rust
// ---------------------------------------------------------------------------

const RUST: &str = r#"use std::sync::Arc;
struct Other;
impl Other {
    fn run(&self) {}
    fn only_here(&self) {}
}
struct Node { x: u32 }
impl Node {
    fn new() -> Self { Node { x: 0 } }
    fn run(&self) {}
    fn with_delai(self, t: u64) -> Self { self }
}
fn make() -> Node { Node::new() }
fn par_annotation() {
    let n: Node = make();
    n.run();
}
fn par_parametre(n: &Node) {
    n.run();
}
fn par_enveloppe(n: Arc<Node>) {
    n.run();
}
fn par_constructeur() {
    let n = Node::new();
    n.run();
}
fn par_litteral() {
    let n = Node { x: 1 };
    n.run();
}
fn delai(t: u64) -> Node {
    let mut node = Node::new();
    node = node.with_delai(t);
    node
}
fn inconnu() {
    let n = make();
    n.run();
    n.only_here();
}
"#;

#[test]
fn rust_le_type_d_une_variable_se_lit_sans_inference() {
    let rels = appels(&[("n.rs", RUST)]);
    appelle(&rels, "par_annotation", "run", "Node");
    appelle(&rels, "par_parametre", "run", "Node");
    appelle(&rels, "par_enveloppe", "run", "Node");
    appelle(&rels, "par_constructeur", "run", "Node");
    appelle(&rels, "par_litteral", "run", "Node");
    appelle(&rels, "delai", "with_delai", "Node");
}

#[test]
fn rust_self_dans_un_impl_est_le_type_de_l_impl() {
    // graph.rs : `let mut graph = Self::new(); graph.connect(…)` — `Self` est
    // le type de l'impl, pas un type à part.
    let src = "struct Other;\nimpl Other {\n    fn connect(&mut self) {}\n}\nstruct Graph;\nimpl Graph {\n    fn new() -> Self { Graph }\n    fn connect(&mut self) {}\n    fn build() -> Self {\n        let mut graph = Self::new();\n        graph.connect();\n        graph\n    }\n}\n";
    let rels = appels(&[("g.rs", src)]);
    appelle(&rels, "build", "connect", "Graph");
}

#[test]
fn rust_sans_type_lisible_pas_de_relation() {
    let rels = appels(&[("n.rs", RUST)]);
    n_appelle_pas(&rels, "inconnu", "run");
    n_appelle_pas(&rels, "inconnu", "only_here");
}

// ---------------------------------------------------------------------------
// TypeScript
// ---------------------------------------------------------------------------

const TS: &str = r#"class Other { run(): void {} }
class Node { run(): void {} }
function make(): any { return null; }
function parParametre(n: Node) { n.run(); }
function parAnnotation() { const n: Node = make(); n.run(); }
function parNew() { const n = new Node(); n.run(); }
function inconnu() { const n = make(); n.run(); }
"#;

#[test]
fn ts_le_type_d_une_variable_se_lit_sans_inference() {
    let rels = appels(&[("n.ts", TS)]);
    appelle(&rels, "parParametre", "run", "Node");
    appelle(&rels, "parAnnotation", "run", "Node");
    appelle(&rels, "parNew", "run", "Node");
    n_appelle_pas(&rels, "inconnu", "run");
}

// ---------------------------------------------------------------------------
// Python
// ---------------------------------------------------------------------------

const PY: &str = r#"class Other:
    def run(self):
        pass

class Node:
    def run(self):
        pass

def make():
    return None

def par_annotation(n: Node):
    n.run()

def par_constructeur():
    n = Node()
    n.run()

def inconnu():
    n = make()
    n.run()
"#;

#[test]
fn python_le_type_d_une_variable_se_lit_sans_inference() {
    let rels = appels(&[("n.py", PY)]);
    appelle(&rels, "par_annotation", "run", "Node");
    appelle(&rels, "par_constructeur", "run", "Node");
    n_appelle_pas(&rels, "inconnu", "run");
}

// ---------------------------------------------------------------------------
// C++
// ---------------------------------------------------------------------------

const CPP: &str = r#"class Other {
 public:
  void run() {}
};
class Node {
 public:
  void run() {}
};
Node* make();
void par_parametre(Node& n) { n.run(); }
void par_declaration() { Node n; n.run(); }
void par_new() { auto n = new Node(); n->run(); }
void inconnu() { auto n = make(); n->run(); }
"#;

#[test]
fn cpp_le_type_d_une_variable_se_lit_sans_inference() {
    let rels = appels(&[("n.cpp", CPP)]);
    appelle(&rels, "par_parametre", "run", "Node");
    appelle(&rels, "par_declaration", "run", "Node");
    appelle(&rels, "par_new", "run", "Node");
    n_appelle_pas(&rels, "inconnu", "run");
}

// ---------------------------------------------------------------------------
// Fermetures et motifs de match : des locales
// ---------------------------------------------------------------------------

#[test]
fn rust_parametres_de_fermeture_et_motifs_de_match_sont_des_locales() {
    let src = "fn count() -> u32 { 0 }\nfn total() -> u32 { 0 }\nfn f(x: Option<u32>) -> u32 {\n    let g = |count: u32| count + 1;\n    match x {\n        Some(total) => g(total),\n        None => 0,\n    }\n}\n";
    let rels = appels(&[("m.rs", src)]);
    n_appelle_pas(&rels, "f", "count");
    n_appelle_pas(&rels, "f", "total");
}

#[test]
fn ts_parametres_de_fermeture_sont_des_locales() {
    let src = "function count(): number { return 0; }\nfunction f(xs: number[]) {\n  return xs.map((count) => count + 1);\n}\n";
    let rels = appels(&[("m.ts", src)]);
    n_appelle_pas(&rels, "f", "count");
}
