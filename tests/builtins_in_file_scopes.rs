//! **Un nom intégré n'est pas une référence du code de premier niveau** (rag3db, 20 novembre 2026) : la relève par le
//! texte d'un scope de fichier écartait les mots vides, pas les noms intégrés du langage. `print(len(x))` au premier
//! niveau d'un module Python gardait `print` et `len` pour références, et les reliait par le nom à un homonyme du
//! projet. Tous les langages qui relèvent leur premier niveau par le texte ; un nom du projet reste une référence.

use std::collections::HashMap;

use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};

fn references_du_premier_niveau(fichier: &str, source: &str) -> Vec<String> {
    let path = format!("/virtual/{fichier}");
    let a = ProjectParser::new(ProjectParserOptions { verbose: false }).parse_project(ParseProjectOptions {
        root: "/virtual".to_string(),
        files: vec![path.clone()],
        content_map: Some(HashMap::from([(path, source.to_string())])),
        resolve_relationships: Some(false),
        resolver_options: None,
    });
    a.files
        .values()
        .flat_map(|f| f.scopes.iter().filter(|s| s.name.starts_with("file_scope")).flat_map(|s| s.identifier_references.iter().map(|r| r.identifier.clone())))
        .collect()
}

#[test]
fn a_builtin_name_is_not_a_reference_of_top_level_code() {
    let cas = [
        ("Python", "app.py", "from lib import charger\n\nx = charger()\nprint(len(x))\n", vec!["print", "len"], "charger"),
        ("JavaScript", "app.js", "import { charger } from './lib';\n\nconst x = charger();\nconsole.log(JSON.stringify(x));\n", vec!["console", "JSON"], "charger"),
        ("Go", "app.go", "package app\n\nvar x = charger()\nvar n = len(x)\n", vec!["len"], "charger"),
    ];
    let mut faux = Vec::new();
    for (langue, fichier, source, integres, du_projet) in cas {
        let refs = references_du_premier_niveau(fichier, source);
        if !refs.iter().any(|r| r == du_projet) {
            faux.push(format!("{langue} : le nom du projet {du_projet} n'est plus une référence : {refs:?}"));
        }
        for n in integres {
            if refs.iter().any(|r| r == n) {
                faux.push(format!("{langue} : le nom intégré {n} est une référence : {refs:?}"));
            }
        }
    }
    assert!(faux.is_empty(), "{faux:#?}");
}
