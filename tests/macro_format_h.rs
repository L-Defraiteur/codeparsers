//! **fmt/format.h : une macro lue comme un corps de fonction** (ticket du 25 octobre 2026, depuis
//! tree-sitter 0.27 et tree-sitter-cpp 0.23.4) : une invocation de macro suivie d'un bloc
//! (`FMT_BEGIN_NAMESPACE`, puis du code) est lue par la grammaire comme une définition de fonction, et
//! codeparsers en faisait un scope `AnonymousFunction`, parent des fonctions qui suivent. Le témoin est le
//! vrai fichier (`tests/fixtures/fmt/format.h`, fmt de spdlog, licence MIT dans `LICENSE.rst`).

use std::collections::HashMap;

use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};

#[test]
fn format_h_aucune_fonction_sous_une_macro_anonyme() {
    let src = include_str!("fixtures/fmt/format.h");
    let chemin = "/virtual/format.h".to_string();
    let a = ProjectParser::new(ProjectParserOptions { verbose: false }).parse_project(ParseProjectOptions {
        root: "/virtual".to_string(),
        files: vec![chemin.clone()],
        content_map: Some(HashMap::from([(chemin, src.to_string())])),
        resolve_relationships: Some(false),
        resolver_options: None,
    });
    let scopes: Vec<_> = a.files.values().flat_map(|fa| fa.scopes.iter()).collect();
    let sous_anonyme: Vec<&str> = scopes.iter().filter(|s| s.parent.as_deref() == Some("AnonymousFunction")).map(|s| s.name.as_str()).collect();
    let anonymes = scopes.iter().filter(|s| s.name == "AnonymousFunction").count();
    eprintln!("format.h : {} scopes, {} AnonymousFunction, {} sous un AnonymousFunction", scopes.len(), anonymes, sous_anonyme.len());
    assert!(sous_anonyme.is_empty(), "{} scopes ont une macro anonyme pour parent : {:?}", sous_anonyme.len(), &sous_anonyme[..sous_anonyme.len().min(12)]);
    for f in ["abort_fuzzing_if", "assume", "bit_cast"] {
        assert!(scopes.iter().any(|s| s.name == f), "{f} est un scope du fichier");
    }
    // Sous `FMT_BEGIN_NAMESPACE namespace detail {`, une fonction a le namespace écrit pour parent.
    assert!(scopes.iter().any(|s| s.name == "do_throw" && s.parent.as_deref() == Some("detail")), "do_throw est sous detail");
    assert!(scopes.iter().any(|s| s.name == "abort_fuzzing_if" && s.parent.as_deref() == Some("detail")), "abort_fuzzing_if est sous detail");
}
