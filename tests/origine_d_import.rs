//! **D'où vient un nom importé** — lu sur la référence, sans rien deviner.
//!
//! En mode « fichier seul », un lien entre fichiers est l'affaire du
//! consommateur (dans rag3weaver, les rendez-vous par le nom). Quand un nom a
//! plusieurs définitions, l'import du fichier qui s'en sert départage : `use
//! crate::estimate::Rate` désigne le `Rate` de `estimate`. Chaque référence
//! porte donc l'import qui amène son nom dans le fichier — ou son
//! qualificatif (`connection::open()` après `use crate::connection`).

use std::collections::HashMap;

use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};
use codeparsers::scope_extraction::types::{IdentifierReference, ImportOrigin};

fn refs(nom: &str, source: &str) -> Vec<(String, IdentifierReference)> {
    let chemin = format!("/virtual/{nom}");
    let a = ProjectParser::new(ProjectParserOptions { verbose: false }).parse_project(ParseProjectOptions {
        root: "/virtual".to_string(),
        files: vec![chemin.clone()],
        content_map: Some(HashMap::from([(chemin, source.to_string())])),
        resolve_relationships: Some(false),
        resolver_options: None,
    });
    a.files.values().flat_map(|fa| fa.scopes.iter().flat_map(|s| s.identifier_references.iter().map(|r| (s.name.clone(), r.clone())))).collect()
}

fn origine(refs: &[(String, IdentifierReference)], scope: &str, ident: &str) -> Option<ImportOrigin> {
    refs.iter().find(|(s, r)| s == scope && r.identifier == ident).unwrap_or_else(|| panic!("pas de {ident} dans {scope}")).1.import_origin.clone()
}

const RUST: &str = "use crate::estimate::Rate;\nuse crate::connection;\nuse std::collections::HashMap as Table;\n\npub fn f(r: Rate) -> u32 {\n    let x = Rate::new();\n    connection::open();\n    let t: Table<u32, u32> = Table::new();\n    local()\n}\n\nfn local() -> u32 {\n    1\n}\n";

#[test]
fn rust_un_nom_importe_porte_son_chemin() {
    let r = refs("lib.rs", RUST);
    let rate = origine(&r, "f", "Rate").expect("Rate vient d'un use");
    assert_eq!((rate.source.as_str(), rate.imported.as_str(), rate.via_qualifier), ("crate::estimate", "Rate", false));
}

#[test]
fn rust_un_qualificatif_importe_porte_son_chemin() {
    let r = refs("lib.rs", RUST);
    let open = origine(&r, "f", "open").expect("connection vient d'un use");
    assert_eq!((open.source.as_str(), open.imported.as_str(), open.via_qualifier), ("crate", "connection", true));
}

#[test]
fn rust_un_alias_rend_le_nom_d_origine() {
    let r = refs("lib.rs", RUST);
    let t = origine(&r, "f", "Table").expect("Table est un alias");
    assert_eq!((t.source.as_str(), t.imported.as_str()), ("std::collections", "HashMap"));
}

#[test]
fn rust_un_nom_du_fichier_n_a_pas_d_origine() {
    let r = refs("lib.rs", RUST);
    assert_eq!(origine(&r, "f", "local"), None);
}

#[test]
fn python_un_alias_rend_le_module_et_le_nom() {
    let r = refs("m.py", "from pkg.models import Base as B\n\n\ndef g():\n    return B()\n");
    let b = origine(&r, "g", "B").expect("B vient d'un import");
    assert_eq!((b.source.as_str(), b.imported.as_str()), ("pkg.models", "Base"));
}
