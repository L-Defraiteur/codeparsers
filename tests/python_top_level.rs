//! **Le code de premier niveau Python a son scope de fichier** (rag3db, 20 novembre 2026), comme en Rust, C, C++,
//! C# et Go : un appel au chargement du module (`register(Catalog)`), une constante construite, le bloc
//! `if __name__ == "__main__":` laissent leurs références. Seul `__init__.py` en avait (lot des paquets).

use std::collections::HashMap;

use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};

#[test]
fn python_top_level_code_has_a_file_scope_with_its_references() {
    let path = "/virtual/app.py".to_string();
    let source = "from lib import Catalog, register\n\ndef helper():\n    return 1\n\nregister(Catalog)\nDEFAULT = helper()\n\nif __name__ == \"__main__\":\n    helper()\n";
    let a = ProjectParser::new(ProjectParserOptions { verbose: false }).parse_project(ParseProjectOptions {
        root: "/virtual".to_string(),
        files: vec![path.clone()],
        content_map: Some(HashMap::from([(path, source.to_string())])),
        resolve_relationships: Some(false),
        resolver_options: None,
    });
    let fichier: Vec<(String, Vec<String>)> = a
        .files
        .values()
        .flat_map(|f| f.scopes.iter().filter(|s| s.name.starts_with("file_scope")).map(|s| (s.name.clone(), s.identifier_references.iter().map(|r| r.identifier.clone()).collect())))
        .collect();
    let refs: Vec<&String> = fichier.iter().flat_map(|(_, r)| r).collect();
    for nom in ["register", "Catalog", "helper"] {
        assert!(refs.iter().any(|r| r.as_str() == nom), "le premier niveau référence {nom} : {fichier:?}");
    }
}

/// **Les liaisons du premier niveau ne sont pas des références** : une variable de boucle (`for ordre in …`), les cibles
/// d'un déballage (`e2, t2 = …`), le nom de module d'un import (`from jevk5.gguf import JevK5GGUF`). Sans elles, le
/// scope de fichier les reliait par le nom à un homonyme d'un autre fichier (une vingtaine d'arêtes sur le crate
/// rag3weaver, 20 novembre 2026). Le nom importé, lui, reste une référence.
#[test]
fn python_top_level_bindings_are_not_references() {
    let path = "/virtual/banc.py".to_string();
    let source = "from jevk5.gguf import JevK5GGUF\n\nmodele = JevK5GGUF()\nfor ordre in (\"a\", \"b\"):\n    print(ordre)\ne2, t2 = modele.charger()\nprint(t2)\n";
    let a = ProjectParser::new(ProjectParserOptions { verbose: false }).parse_project(ParseProjectOptions {
        root: "/virtual".to_string(),
        files: vec![path.clone()],
        content_map: Some(HashMap::from([(path, source.to_string())])),
        resolve_relationships: Some(false),
        resolver_options: None,
    });
    let refs: Vec<String> = a
        .files
        .values()
        .flat_map(|f| f.scopes.iter().filter(|s| s.name.starts_with("file_scope")).flat_map(|s| s.identifier_references.iter().map(|r| r.identifier.clone())))
        .collect();
    assert!(refs.iter().any(|r| r == "JevK5GGUF"), "le nom importé reste une référence : {refs:?}");
    let faux: Vec<&str> = ["ordre", "e2", "t2", "jevk5", "gguf"].into_iter().filter(|n| refs.iter().any(|r| r == n)).collect();
    assert!(faux.is_empty(), "des liaisons du premier niveau passent pour des références : {faux:?} dans {refs:?}");
}
