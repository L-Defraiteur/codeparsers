//! **Le type d'un appel se lit aussi par un champ déclaré ou un type de
//! retour déclaré** — toujours sans inférence.
//!
//! - Un champ : `self.store.get()`, `this.store.get()`, `v.store.get()` avec
//!   `v` typé ; le type de `store` est écrit dans la struct ou la classe. Un
//!   niveau de champ, pas plus.
//! - Un retour : `let s = make_store(); s.get()` ou `make_store().get()`,
//!   quand `make_store` se résout sans ambiguïté et déclare son type de
//!   retour ; `try_store()?` déballe un `Result` ou une `Option`. Un maillon,
//!   pas de chaîne.
//! - `Arc`, `Box`, `Rc` et `&` se traversent ; tout autre générique garde son
//!   nom (`Option<Store>` est une `Option`).
//!
//! Le type concurrent (`Other`) est toujours déclaré avant le bon.

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
pub struct Other;
impl Other {
    pub fn get(&self) {}
}
pub struct Store;
impl Store {
    pub fn get(&self) {}
}
pub struct Svc {
    store: Store,
    shared: Arc<Store>,
    peut_etre: Option<Store>,
}
impl Svc {
    fn par_champ(&self) {
        self.store.get();
    }
    fn par_enveloppe(&self) {
        self.shared.get();
    }
    fn par_option(&self) {
        self.peut_etre.get();
    }
}
fn make_store() -> Store {
    Store
}
fn try_store() -> Result<Store, String> {
    Ok(Store)
}
fn par_retour() {
    let s = make_store();
    s.get();
}
fn par_retour_try() -> Result<(), String> {
    let s = try_store()?;
    s.get();
    Ok(())
}
fn par_chaine() {
    make_store().get();
}
fn par_variable_et_champ(v: &Svc) {
    v.store.get();
}
fn deux_niveaux(v: &Svc) {
    v.store.inner.get();
}
"#;

#[test]
fn rust_le_type_d_un_champ_declare_se_lit() {
    let rels = appels(&[("s.rs", RUST)]);
    appelle(&rels, "par_champ", "get", "Store");
    appelle(&rels, "par_enveloppe", "get", "Store");
    appelle(&rels, "par_variable_et_champ", "get", "Store");
}

#[test]
fn rust_le_type_de_retour_declare_se_lit() {
    let rels = appels(&[("s.rs", RUST)]);
    appelle(&rels, "par_retour", "get", "Store");
    appelle(&rels, "par_retour_try", "get", "Store");
    appelle(&rels, "par_chaine", "get", "Store");
}

#[test]
fn rust_ce_qui_ne_se_lit_pas_ne_donne_rien() {
    let rels = appels(&[("s.rs", RUST)]);
    n_appelle_pas(&rels, "par_option", "get");
    n_appelle_pas(&rels, "deux_niveaux", "get");
}

#[test]
fn rust_une_fonction_ambigue_ne_donne_pas_de_type() {
    let autre = "pub fn make_store() -> super::Other {\n    super::Other\n}\n";
    let rels = appels(&[("s.rs", RUST), ("autre.rs", autre)]);
    n_appelle_pas(&rels, "par_retour", "get");
    n_appelle_pas(&rels, "par_chaine", "get");
}

// ---------------------------------------------------------------------------
// TypeScript
// ---------------------------------------------------------------------------

const TS: &str = r#"class Other { get(): void {} }
class Store { get(): void {} }
class Svc {
  private store: Store;
  run() { this.store.get(); }
}
function makeStore(): Store { return new Store(); }
function parRetour() { const s = makeStore(); s.get(); }
function parChaine() { makeStore().get(); }
"#;

#[test]
fn ts_champ_et_retour_declares() {
    let rels = appels(&[("s.ts", TS)]);
    appelle(&rels, "run", "get", "Store");
    appelle(&rels, "parRetour", "get", "Store");
    appelle(&rels, "parChaine", "get", "Store");
}

// ---------------------------------------------------------------------------
// Python
// ---------------------------------------------------------------------------

const PY: &str = r#"class Other:
    def get(self):
        pass

class Store:
    def get(self):
        pass

class Svc:
    store: Store

    def __init__(self):
        self.autre = Store()

    def par_annotation(self):
        self.store.get()

    def par_init(self):
        self.autre.get()

def make_store() -> Store:
    return Store()

def par_retour():
    s = make_store()
    s.get()
"#;

#[test]
fn python_champs_annotes_ou_construits_et_retours_annotes() {
    let rels = appels(&[("s.py", PY)]);
    appelle(&rels, "par_annotation", "get", "Store");
    appelle(&rels, "par_init", "get", "Store");
    appelle(&rels, "par_retour", "get", "Store");
}

// ---------------------------------------------------------------------------
// C++
// ---------------------------------------------------------------------------

const CPP: &str = r#"class Other {
 public:
  void get() {}
};
class Store {
 public:
  void get() {}
};
class Svc {
  Store store;
 public:
  void implicite() { store.get(); }
  void explicite() { this->store.get(); }
};
Store make() { return Store(); }
void par_retour() { auto s = make(); s.get(); }
"#;

#[test]
fn cpp_champ_implicite_explicite_et_retour() {
    let rels = appels(&[("s.cpp", CPP)]);
    appelle(&rels, "implicite", "get", "Store");
    appelle(&rels, "explicite", "get", "Store");
    appelle(&rels, "par_retour", "get", "Store");
}

// ---------------------------------------------------------------------------
// Les enveloppes
// ---------------------------------------------------------------------------

#[test]
fn rust_box_rc_arc_et_la_reference_se_traversent_mutex_non() {
    let src = r#"use std::rc::Rc;
use std::sync::{Arc, Mutex};
pub struct Other;
impl Other {
    pub fn get(&self) {}
}
pub struct Store;
impl Store {
    pub fn get(&self) {}
}
pub struct Svc {
    boite: Box<Store>,
    verrou: Arc<Mutex<Store>>,
}
impl Svc {
    fn par_box(&self) {
        self.boite.get();
    }
    fn par_mutex(&self) {
        self.verrou.get();
    }
}
fn par_rc(r: Rc<Store>) {
    r.get();
}
fn par_reference(r: &mut Store) {
    r.get();
}
"#;
    let rels = appels(&[("e.rs", src)]);
    appelle(&rels, "par_box", "get", "Store");
    appelle(&rels, "par_rc", "get", "Store");
    appelle(&rels, "par_reference", "get", "Store");
    // `Arc<Mutex<Store>>` est un `Mutex` une fois `Arc` traversé ; atteindre
    // le `Store` demande `.lock()`, un maillon de plus : pas de relation.
    n_appelle_pas(&rels, "par_mutex", "get");
}
