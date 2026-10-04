//! **Une variable locale n'est pas un usage.** En Rust, les noms liés par un
//! motif — `let`, `match`, `if let`, `while let`, `for`, les paramètres d'une
//! fermeture — sont des locales de leur scope. Ils sortaient comme des
//! références « inconnues », que le rendez-vous de rag3weaver reliait à la
//! seule fonction du même nom : `read_sse` « utilisait » `fn s` de code.rs
//! parce qu'il écrit `let s = match … { Some(s) => *s, … }`.

use std::collections::HashMap;

use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};

const SOURCE: &str = "pub fn f(x: Option<u32>, v: Vec<u32>) -> u32 {\n    let s = match x {\n        Some(s) => s,\n        None => 0,\n    };\n    let (a, mut b) = (1, 2);\n    b += a;\n    if let Some(y) = x {\n        b += y;\n    }\n    for w in &v {\n        b += w;\n    }\n    let total: u32 = v.iter().map(|p| p + 1).sum();\n    s + b + total + g()\n}\n\nfn g() -> u32 {\n    1\n}\n";

fn references_de(nom: &str) -> Vec<String> {
    let chemin = "/virtual/b.rs".to_string();
    let a = ProjectParser::new(ProjectParserOptions { verbose: false }).parse_project(ParseProjectOptions {
        root: "/virtual".to_string(),
        files: vec![chemin.clone()],
        content_map: Some(HashMap::from([(chemin, SOURCE.to_string())])),
        resolve_relationships: Some(false),
        resolver_options: None,
    });
    a.files
        .values()
        .flat_map(|f| f.scopes.iter().filter(|s| s.name == nom).flat_map(|s| s.identifier_references.iter().map(|r| r.identifier.clone())))
        .collect()
}

#[test]
fn les_noms_lies_par_un_motif_ne_sont_pas_des_references() {
    let refs = references_de("f");
    for local in ["s", "a", "b", "y", "w", "p", "total"] {
        assert!(!refs.contains(&local.to_string()), "« {local} » est une locale : {refs:?}");
    }
    assert!(refs.contains(&"g".to_string()), "un vrai appel reste une référence : {refs:?}");
}
