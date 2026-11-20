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
