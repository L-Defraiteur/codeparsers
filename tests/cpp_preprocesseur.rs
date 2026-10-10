//! **Une directive du préprocesseur ne nomme rien du projet.**
//!
//! Le scope de fichier lit ses noms sur le texte : `#pragma once` et
//! `#include "storage/table/table.h"` rendaient des références `pragma`,
//! `once`, `include`, `storage`, `table`, `h` — 45 000 « noms seuls » et
//! 13 000 imports non résolus sur le C++ de rag3db (banc du 10 octobre).
//! Le `#include` reste une référence d'import, relevée sur l'arbre.

use std::collections::{BTreeSet, HashMap};

use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};

#[test]
fn une_ligne_de_directive_ne_nomme_rien() {
    let src = "#pragma once\n\n#include <vector>\n#include \"storage/table/table.h\"\n#define TAILLE 4\n#if defined(NDEBUG)\n#endif\n\nnamespace rag3db {\nclass Table;\nint largeur(Table* t);\n}\n";
    let chemin = "/virtual/node_table.h".to_string();
    let r = ProjectParser::new(ProjectParserOptions { verbose: false }).parse_project(ParseProjectOptions {
        root: "/virtual".to_string(),
        files: vec![chemin.clone()],
        content_map: Some(HashMap::from([(chemin, src.to_string())])),
        resolve_relationships: Some(false),
        resolver_options: None,
    });
    let fa = r.files.values().next().expect("un fichier");
    let noms: BTreeSet<&str> = fa.scopes.iter().flat_map(|s| s.identifier_references.iter().map(|x| x.identifier.as_str())).collect();
    for directive in ["pragma", "once", "include", "h", "define", "endif", "defined"] {
        assert!(!noms.contains(directive), "« {directive} » relevé : {noms:?}");
    }
    let imports: Vec<&str> = fa.scopes.iter().flat_map(|s| s.import_references.iter().map(|i| i.source.as_str())).collect();
    assert!(imports.iter().any(|i| i.contains("storage/table/table.h")), "le #include reste un import : {imports:?}");
}
