//! **Un `pub use` est une réexportation** (lot 2 bis de la stèle du
//! produit, 11 octobre 2026 ; décision du 4 octobre) : l'`ImportReference`
//! le dit (`reexport`), pour que le consommateur suive `crate::dataflow::X`
//! jusqu'à la définition de `X` dans `dataflow/runtime.rs`. Un `use` privé
//! ne l'est pas.

use std::collections::HashMap;

use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};

/// (lié, module, réexporté) des imports du fichier.
fn imports(source: &str) -> Vec<(Option<String>, Option<String>, bool)> {
    let chemin = "/virtual/src/dataflow/mod.rs".to_string();
    let a = ProjectParser::new(ProjectParserOptions { verbose: false }).parse_project(ParseProjectOptions {
        root: "/virtual".to_string(),
        files: vec![chemin.clone()],
        content_map: Some(HashMap::from([(chemin, source.to_string())])),
        resolve_relationships: Some(false),
        resolver_options: None,
    });
    let mut v: Vec<_> = a
        .files
        .values()
        .flat_map(|fa| fa.import_references.iter())
        .map(|i| (i.alias.clone(), i.module_path.clone(), i.reexport))
        .collect();
    v.sort();
    v.dedup();
    v
}

#[test]
fn un_pub_use_est_une_reexportation() {
    let src = "pub mod runtime;\npub use runtime::DataflowEvent;\npub(crate) use self::runtime::Graine;\nuse crate::autre::Prive;\npub fn f(_e: DataflowEvent, _g: Graine, _p: Prive) {}\n";
    let v = imports(src);
    eprintln!("{v:?}");
    let de = |nom: &str| v.iter().find(|(a, _, _)| a.as_deref() == Some(nom)).cloned().unwrap_or_else(|| panic!("{nom} absent : {v:?}"));
    assert!(de("DataflowEvent").2, "pub use : réexporté — {v:?}");
    assert_eq!(de("DataflowEvent").1.as_deref(), Some("runtime"));
    assert!(de("Graine").2, "pub(crate) use : réexporté — {v:?}");
    assert!(!de("Prive").2, "un use privé ne l'est pas — {v:?}");
}
