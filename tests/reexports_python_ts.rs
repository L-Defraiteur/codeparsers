//! **Les réexportations de Python et de TypeScript** (rag3db, 20 novembre 2026) : un import nommé d'un `__init__.py`
//! est la réexportation du paquet (la convention du langage), et `export { X } from './x'` / `export * from './x'`
//! en sont une en TypeScript. Seul Rust (`pub use`) en marquait : `from monpaquet import Catalog` ne suivait pas
//! `__init__.py` jusqu'à `lib.py`.

use std::collections::HashMap;

use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};
use codeparsers::scope_extraction::types::ImportReference;

fn imports(file: &str, source: &str) -> Vec<ImportReference> {
    let path = format!("/virtual/{file}");
    let a = ProjectParser::new(ProjectParserOptions { verbose: false }).parse_project(ParseProjectOptions {
        root: "/virtual".to_string(),
        files: vec![path.clone()],
        content_map: Some(HashMap::from([(path, source.to_string())])),
        resolve_relationships: Some(false),
        resolver_options: None,
    });
    a.files.values().flat_map(|f| f.import_references.clone()).collect()
}

#[test]
fn python_and_typescript_mark_their_reexports() {
    let mut faux = Vec::new();
    let init = imports("monpaquet/__init__.py", "from .lib import Catalog\nfrom monpaquet.outils import Outil as Tool\n");
    for (nom, source) in [("Catalog", ".lib"), ("Outil", "monpaquet.outils")] {
        if !init.iter().any(|i| i.imported == nom && i.source == source && i.reexport) {
            faux.push(format!("Python __init__.py : {nom} depuis {source} n'est pas une réexportation : {init:?}"));
        }
    }
    let module = imports("monpaquet/usage.py", "from .lib import Catalog\n");
    if module.iter().any(|i| i.reexport) {
        faux.push(format!("Python : un import d'un module ordinaire n'est pas une réexportation : {module:?}"));
    }
    let index = imports("src/index.ts", "export { Catalog } from './catalog';\nexport { A as B } from './a';\nexport * from './all';\n");
    for (nom, source) in [("Catalog", "./catalog"), ("A", "./a"), ("*", "./all")] {
        if !index.iter().any(|i| i.imported == nom && i.source == source && i.reexport) {
            faux.push(format!("TypeScript : {nom} depuis {source} n'est pas une réexportation : {index:?}"));
        }
    }
    if !index.iter().any(|i| i.imported == "A" && i.alias.as_deref() == Some("B")) {
        faux.push(format!("TypeScript : export {{ A as B }} garde son alias : {index:?}"));
    }
    assert!(faux.is_empty(), "{faux:#?}");
}
