//! **Un `macro_rules!` est un scope** (T4 de la session recherche, 11
//! octobre 2026). Sans scope, le corps d'une macro se lisait dans le scope
//! du fichier (`macro_rules`, `expr`, `x` y devenaient des références), et
//! l'appel `disons!(41)` n'avait pas de cible : l'impact ne voyait pas au
//! travers d'une macro. La macro est un scope `Function` nommé d'après
//! elle, modificateur `macro`, dont les références sont celles du corps de
//! ses règles (relues comme dans les arguments d'une macro : `aide` y est
//! un appel qualifié `crate::fermetures`).

use std::collections::HashMap;

use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};
use codeparsers::scope_extraction::types::{ScopeInfo, ScopeInfoType, UsageKind};

const SRC: &str = "macro_rules! disons {\n    ($x:expr) => { crate::fermetures::aide($x) };\n    ($x:expr, $y:ident) => { $y.jouer(); aide2($x) };\n}\npub fn par_macro() -> i32 { disons!(41) }\n";

fn scopes() -> Vec<ScopeInfo> {
    let chemin = "/virtual/macros.rs".to_string();
    let a = ProjectParser::new(ProjectParserOptions { verbose: false }).parse_project(ParseProjectOptions {
        root: "/virtual".to_string(),
        files: vec![chemin.clone()],
        content_map: Some(HashMap::from([(chemin, SRC.to_string())])),
        resolve_relationships: Some(false),
        resolver_options: None,
    });
    a.files.into_values().flat_map(|fa| fa.scopes).collect()
}

#[test]
fn la_macro_est_un_scope_et_lit_le_corps_de_ses_regles() {
    let s = scopes();
    let m = s.iter().find(|x| x.name == "disons").expect("un scope pour la macro");
    assert_eq!(m.r#type, ScopeInfoType::Function);
    assert!(m.modifiers.iter().any(|x| x == "macro"), "{:?}", m.modifiers);
    let appels: Vec<(String, Option<String>)> = m
        .identifier_references
        .iter()
        .filter(|r| r.usage == Some(UsageKind::Call))
        .map(|r| (r.identifier.clone(), r.qualifier.clone()))
        .collect();
    assert!(appels.contains(&("aide".to_string(), Some("crate::fermetures".to_string()))), "{appels:?}");
    assert!(appels.iter().any(|(n, _)| n == "aide2"), "les règles suivantes aussi : {appels:?}");
    let noms: Vec<&str> = m.identifier_references.iter().map(|r| r.identifier.as_str()).collect();
    assert!(!noms.iter().any(|n| matches!(*n, "expr" | "ident" | "x" | "y" | "macro_rules")), "ni spécificateur ni métavariable : {noms:?}");
}

#[test]
fn le_scope_du_fichier_ne_lit_plus_le_corps_de_la_macro() {
    let s = scopes();
    let fichier: Vec<&str> =
        s.iter().filter(|x| x.r#type == ScopeInfoType::Module).flat_map(|x| x.identifier_references.iter().map(|r| r.identifier.as_str())).collect();
    assert!(!fichier.iter().any(|n| matches!(*n, "macro_rules" | "expr" | "aide" | "fermetures")), "{fichier:?}");
    let site: Vec<&str> =
        s.iter().filter(|x| x.name == "par_macro").flat_map(|x| x.identifier_references.iter().map(|r| r.identifier.as_str())).collect();
    assert!(site.contains(&"disons"), "le site d'expansion appelle la macro : {site:?}");
}
