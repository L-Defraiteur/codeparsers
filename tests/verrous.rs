//! **Les verrous qu'un scope prend** : une référence au champ mutex, de genre
//! `Lock` (exclusif) ou `SharedLock` (partagé), dont le qualificatif est le
//! propriétaire (`this` / `self` pour un membre de la classe englobante).
//! Le consommateur en fait la relation « prend le verrou `Classe::champ` »,
//! le propriétaire typé comme tout receveur.
//!
//! Banc du 10 octobre : 188 gardes RAII dans le moteur C++, aucune vue.
//! Une garde RAII est certaine (`unique_lock`, `lock_guard`, `scoped_lock`
//! exclusifs, `shared_lock` partagé) ; un appel `.lock()`, `.read()`,
//! `.write()` ne compte que si le champ est déclaré mutex dans le fichier
//! (`weak_ptr::lock()` n'est pas un verrou).

use std::collections::{BTreeSet, HashMap};

use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};
use codeparsers::scope_extraction::types::UsageKind;

/// (scope, champ, propriétaire, partagé) des verrous relevés.
fn verrous(nom: &str, source: &str) -> BTreeSet<(String, String, String, bool)> {
    let chemin = format!("/virtual/{nom}");
    let a = ProjectParser::new(ProjectParserOptions { verbose: false }).parse_project(ParseProjectOptions {
        root: "/virtual".to_string(),
        files: vec![chemin.clone()],
        content_map: Some(HashMap::from([(chemin, source.to_string())])),
        resolve_relationships: Some(false),
        resolver_options: None,
    });
    a.files
        .values()
        .flat_map(|fa| fa.scopes.iter())
        .flat_map(|s| {
            s.identifier_references.iter().filter_map(move |r| match r.usage {
                Some(UsageKind::Lock) | Some(UsageKind::SharedLock) => Some((
                    s.name.clone(),
                    r.identifier.clone(),
                    r.qualifier.clone().unwrap_or_default(),
                    r.usage == Some(UsageKind::SharedLock),
                )),
                _ => None,
            })
        })
        .collect()
}

fn v(scope: &str, champ: &str, proprietaire: &str, partage: bool) -> (String, String, String, bool) {
    (scope.to_string(), champ.to_string(), proprietaire.to_string(), partage)
}

const CPP: &str = r#"#include <mutex>
#include <shared_mutex>

class Index {
public:
    void insert(int k) {
        std::unique_lock lck{mtx};
    }
    void read(int k) const {
        std::shared_lock lck{mtx};
    }
    void both(Other* o) {
        std::lock_guard<std::mutex> g(this->mtx);
        std::scoped_lock s(o->m1, other.m2);
        std::unique_lock d{mtx, std::defer_lock};
        std::mutex local;
        std::lock_guard l(local);
    }
    void essai() {
        if (mtx.try_lock()) {
        }
        auto p = weak.lock();
    }
private:
    std::shared_mutex mtx;
    Other other;
    std::weak_ptr<Other> weak;
};
"#;

#[test]
fn cpp_les_gardes_et_les_appels_sur_un_mutex() {
    let r = verrous("index.h", CPP);
    eprintln!("{r:#?}");
    let attendu: BTreeSet<_> = [
        v("insert", "mtx", "this", false),
        v("read", "mtx", "this", true),
        v("both", "mtx", "this", false),
        v("both", "m1", "o", false),
        v("both", "m2", "other", false),
        v("essai", "mtx", "this", false),
    ]
    .into_iter()
    .collect();
    assert_eq!(r, attendu, "un mutex local n'est pas un champ ; weak_ptr::lock() n'est pas un verrou ; std::defer_lock n'est pas un mutex");
}

const RUST: &str = "use std::sync::{Mutex, RwLock, Weak};\n\npub struct Store {\n    inner: Mutex<u32>,\n    idx: RwLock<u32>,\n    lien: Weak<u32>,\n}\n\nimpl Store {\n    fn a(&self) {\n        let g = self.inner.lock().unwrap();\n    }\n    fn b(&self) {\n        let r = self.idx.read().unwrap();\n        let w = self.idx.write().unwrap();\n        let l = self.lien.upgrade();\n    }\n}\n";

#[test]
fn rust_un_champ_mutex_ou_rwlock() {
    let r = verrous("store.rs", RUST);
    eprintln!("{r:#?}");
    let attendu: BTreeSet<_> = [v("a", "inner", "self", false), v("b", "idx", "self", true), v("b", "idx", "self", false)].into_iter().collect();
    assert_eq!(r, attendu);
}
