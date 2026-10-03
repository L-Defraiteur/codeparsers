//! **Chaque relation dit comment on se sert de sa cible, et à quelle ligne.**
//!
//! Le genre d'usage (`call`, `type`, `import`, `inheritance`, `other`) est lu
//! sur l'AST au moment de l'extraction, là où la grammaire le dit, et non
//! deviné après coup sur le texte de la ligne. Une relation porte la liste de
//! ses sites : un par occurrence, ou plusieurs quand le résolveur fusionne
//! les références d'un scope vers une même cible.
//!
//! Ces tests n'assertent que le genre et la ligne : les relations elles-mêmes
//! (types, nombre) sont l'affaire de `relationships.rs` et ne changent pas.

use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};
use codeparsers::relationship_resolution::types::{RelationshipType, ResolvedRelationship};
use codeparsers::scope_extraction::types::UsageKind;
use std::collections::HashMap;

fn relations(fichiers: &[(&str, &str)]) -> Vec<ResolvedRelationship> {
    let racine = "/virtual";
    let mut contenus = HashMap::new();
    let mut chemins = Vec::new();
    for (nom, source) in fichiers {
        let chemin = format!("{racine}/{nom}");
        contenus.insert(chemin.clone(), source.to_string());
        chemins.push(chemin);
    }
    ProjectParser::new(ProjectParserOptions { verbose: false })
        .parse_project(ParseProjectOptions {
            root: racine.to_string(),
            files: chemins,
            content_map: Some(contenus),
            resolve_relationships: Some(true),
            resolver_options: None,
        })
        .relationships
        .map_or_else(Vec::new, |r| r.relationships)
}

/// Les sites (genre, ligne) de toutes les relations `de → vers` du type donné.
fn sites(rels: &[ResolvedRelationship], de: &str, vers: &str, t: RelationshipType) -> Vec<(UsageKind, Option<usize>)> {
    let choisies: Vec<_> = rels.iter().filter(|r| r.from_name == de && r.to_name == vers && r.r#type == t).collect();
    assert!(!choisies.is_empty(), "aucune relation {t:?} {de} → {vers} ; relations : {:#?}",
        rels.iter().map(|r| format!("{:?} {} → {}", r.r#type, r.from_name, r.to_name)).collect::<Vec<_>>());
    choisies.iter()
        .flat_map(|r| r.metadata.as_ref().map(|m| m.sites.clone()).unwrap_or_default())
        .map(|s| (s.usage, s.line))
        .collect()
}

fn a_le_site(rels: &[ResolvedRelationship], de: &str, vers: &str, t: RelationshipType, usage: UsageKind, ligne: usize) {
    let s = sites(rels, de, vers, t.clone());
    assert!(s.contains(&(usage.clone(), Some(ligne))),
        "{t:?} {de} → {vers} devrait avoir le site ({usage:?}, ligne {ligne}) ; sites : {s:?}");
}

/// Toute relation qui n'est pas structurelle porte au moins un site, et chaque
/// site d'une référence d'AST a une ligne.
fn chaque_usage_a_un_site(rels: &[ResolvedRelationship]) {
    for r in rels {
        let structurelle = matches!(r.r#type,
            RelationshipType::PARENTOF | RelationshipType::HASPARENT | RelationshipType::DEFINEDIN);
        let s = r.metadata.as_ref().map(|m| m.sites.clone()).unwrap_or_default();
        if structurelle {
            assert!(s.is_empty(), "{:?} {} → {} est structurelle, pas un usage : {s:?}", r.r#type, r.from_name, r.to_name);
        } else {
            assert!(!s.is_empty(), "{:?} {} → {} n'a aucun site", r.r#type, r.from_name, r.to_name);
        }
    }
}

// ---------------------------------------------------------------------------
// Rust
// ---------------------------------------------------------------------------

const RUST: &str = r#"struct Point { x: i32 }
trait Shape { fn area(&self) -> f64; }
struct Circle { center: Point }
impl Shape for Circle {
    fn area(&self) -> f64 { 0.0 }
}
fn make() -> Point { Point { x: 1 } }
fn take_or_clone<T>(v: T) -> T { v }
fn run() {
    let p: Point = make();
    let q = take_or_clone::<i32>(1);
}
"#;

#[test]
fn rust_appel_type_et_implementation() {
    let rels = relations(&[("geo.rs", RUST)]);
    a_le_site(&rels, "run", "make", RelationshipType::CONSUMES, UsageKind::Call, 10);
    a_le_site(&rels, "run", "Point", RelationshipType::CONSUMES, UsageKind::Type, 10);
    a_le_site(&rels, "run", "take_or_clone", RelationshipType::CONSUMES, UsageKind::Call, 11);
    a_le_site(&rels, "Circle", "Shape", RelationshipType::IMPLEMENTS, UsageKind::Inheritance, 4);
    chaque_usage_a_un_site(&rels);
}

#[test]
fn rust_un_appel_dans_les_arguments_d_une_macro_est_un_appel() {
    let src = "struct B;\nimpl B {\n    fn taken(&self) -> bool { true }\n    fn show(&self) {\n        println!(\"{}\", self.taken());\n    }\n}\n";
    let rels = relations(&[("b.rs", src)]);
    a_le_site(&rels, "show", "taken", RelationshipType::CONSUMES, UsageKind::Call, 5);
}

#[test]
fn rust_le_champ_d_une_structure_est_un_usage_de_type() {
    let rels = relations(&[("geo.rs", RUST)]);
    a_le_site(&rels, "Circle", "Point", RelationshipType::CONSUMES, UsageKind::Type, 3);
}

// ---------------------------------------------------------------------------
// TypeScript / JavaScript
// ---------------------------------------------------------------------------

const TS: &str = r#"import { readFile } from "fs";
class Base {}
interface Shape {}
class Circle extends Base implements Shape {
    area(): number { return helper(); }
}
function helper(): number { readFile("x"); return 1; }
function build(c: Circle): Circle {
    const x = new Circle();
    return x;
}
"#;

#[test]
fn ts_heritage_appel_instanciation_et_import() {
    let rels = relations(&[("geo.ts", TS)]);
    a_le_site(&rels, "Circle", "Base", RelationshipType::INHERITSFROM, UsageKind::Inheritance, 4);
    a_le_site(&rels, "Circle", "Shape", RelationshipType::IMPLEMENTS, UsageKind::Inheritance, 4);
    a_le_site(&rels, "area", "helper", RelationshipType::CONSUMES, UsageKind::Call, 5);
    a_le_site(&rels, "build", "Circle", RelationshipType::CONSUMES, UsageKind::Call, 9);
    a_le_site(&rels, "helper", "fs", RelationshipType::USESLIBRARY, UsageKind::Import, 1);
    chaque_usage_a_un_site(&rels);
}

#[test]
fn ts_un_type_de_signature_est_un_usage_de_type() {
    let rels = relations(&[("sig.ts", "class Point {}\nfunction norm(p: Point): number { return 0; }\n")]);
    a_le_site(&rels, "norm", "Point", RelationshipType::CONSUMES, UsageKind::Type, 2);
}

#[test]
fn js_appel_simple() {
    let rels = relations(&[("app.js", "function a() {}\nfunction b() {\n  a();\n}\n")]);
    a_le_site(&rels, "b", "a", RelationshipType::CONSUMES, UsageKind::Call, 3);
    chaque_usage_a_un_site(&rels);
}

#[test]
fn js_une_ecriture_n_est_pas_un_appel() {
    let rels = relations(&[("app.js", "let busy = false;\nfunction setBusy(v) {\n  busy = v;\n}\n")]);
    let s = sites(&rels, "setBusy", "busy", RelationshipType::CONSUMES);
    assert!(!s.is_empty() && s.iter().all(|(u, l)| *u == UsageKind::Other && *l == Some(3)), "busy = v est une écriture : {s:?}");
}

// ---------------------------------------------------------------------------
// Python
// ---------------------------------------------------------------------------

const PY: &str = r#"import os

class Base:
    pass

class Child(Base):
    def run(self):
        return helper()

def helper() -> Base:
    os.getcwd()
    return Base()
"#;

#[test]
fn python_heritage_appel_et_import() {
    let rels = relations(&[("mod.py", PY)]);
    a_le_site(&rels, "Child", "Base", RelationshipType::INHERITSFROM, UsageKind::Inheritance, 6);
    a_le_site(&rels, "helper", "Base", RelationshipType::CONSUMES, UsageKind::Call, 12);
    // Pas de USES_LIBRARY en Python : l'extracteur d'imports marque tout
    // import comme local (`is_local: true` en dur). Un trou connu, hors de ce lot.
    // Python voit l'appel deux fois (comme appel et comme identifiant) : les
    // deux relations disent « appel », aucune ne dit « autre ».
    let s = sites(&rels, "run", "helper", RelationshipType::CONSUMES);
    assert!(!s.is_empty() && s.iter().all(|(u, l)| *u == UsageKind::Call && *l == Some(8)), "run → helper : {s:?}");
    chaque_usage_a_un_site(&rels);
}

#[test]
fn python_une_annotation_est_un_usage_de_type() {
    let rels = relations(&[("ann.py", "class Point:\n    pass\n\ndef norm(p: Point) -> int:\n    return 0\n")]);
    a_le_site(&rels, "norm", "Point", RelationshipType::CONSUMES, UsageKind::Type, 4);
}

// ---------------------------------------------------------------------------
// C, C++, C#, Go
// ---------------------------------------------------------------------------

#[test]
fn c_appel() {
    let rels = relations(&[("main.c", "int helper(void) { return 1; }\nint main(void) {\n  return helper();\n}\n")]);
    a_le_site(&rels, "main", "helper", RelationshipType::CONSUMES, UsageKind::Call, 3);
    chaque_usage_a_un_site(&rels);
}

#[test]
fn cpp_heritage_et_appel() {
    let src = "class Base { public: virtual void f(); };\nclass Derived : public Base {\n public:\n  void g() { helper(); }\n};\nvoid helper() {}\n";
    let rels = relations(&[("geo.cpp", src)]);
    a_le_site(&rels, "Derived", "Base", RelationshipType::INHERITSFROM, UsageKind::Inheritance, 2);
    a_le_site(&rels, "g", "helper", RelationshipType::CONSUMES, UsageKind::Call, 4);
    chaque_usage_a_un_site(&rels);
}

#[test]
fn csharp_heritage_appel_et_instanciation() {
    let src = "class Index {}\nclass Item : Index {}\nclass Svc {\n  void Run() {\n    var x = new Item();\n    Helper();\n  }\n  void Helper() {}\n}\n";
    let rels = relations(&[("svc.cs", src)]);
    a_le_site(&rels, "Item", "Index", RelationshipType::INHERITSFROM, UsageKind::Inheritance, 2);
    a_le_site(&rels, "Run", "Item", RelationshipType::CONSUMES, UsageKind::Call, 5);
    a_le_site(&rels, "Run", "Helper", RelationshipType::CONSUMES, UsageKind::Call, 6);
    chaque_usage_a_un_site(&rels);
}

#[test]
fn go_appel_type_et_inclusion() {
    let src = "package main\n\ntype Store struct{ n int }\n\ntype Outer struct {\n\tStore\n}\n\nfunc NewStore() *Store { return &Store{} }\n\nfunc Use() {\n\tt := NewStore()\n\t_ = t\n}\n";
    let rels = relations(&[("main.go", src)]);
    a_le_site(&rels, "Use", "NewStore", RelationshipType::CONSUMES, UsageKind::Call, 12);
    a_le_site(&rels, "NewStore", "Store", RelationshipType::CONSUMES, UsageKind::Type, 9);
    a_le_site(&rels, "Outer", "Store", RelationshipType::INHERITSFROM, UsageKind::Inheritance, 5);
    chaque_usage_a_un_site(&rels);
}

// ---------------------------------------------------------------------------
// Fusion des sites
// ---------------------------------------------------------------------------

#[test]
fn une_relation_fusionnee_garde_tous_ses_sites() {
    // `Point` est défini dans un autre fichier : les références de `run` y sont
    // résolues par nom et fusionnées en une seule relation. Ses deux sites
    // (le paramètre à la ligne 1, la variable à la ligne 2) restent tous deux.
    let rels = relations(&[
        ("point.rs", "pub struct Point;\n"),
        ("run.rs", "fn run(a: Point) {\n    let b: Point = a;\n}\n"),
    ]);
    let n = rels.iter().filter(|r| r.from_name == "run" && r.to_name == "Point" && r.r#type == RelationshipType::CONSUMES).count();
    assert_eq!(n, 1, "une seule relation, comme avant");
    let s = sites(&rels, "run", "Point", RelationshipType::CONSUMES);
    assert!(s.contains(&(UsageKind::Type, Some(1))) && s.contains(&(UsageKind::Type, Some(2))), "sites : {s:?}");
}
