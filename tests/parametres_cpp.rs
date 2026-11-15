//! **Un paramètre C++ par pointeur ou par référence garde son nom** (ticket du 15 novembre 2026) :
//! `extract_c_parameters` ne cherchait le nom que parmi les enfants directs du `parameter_declaration`, et un
//! paramètre `T* p`, `T& r` ou `T&& r` (le nom dans un `pointer_declarator` ou un `reference_declarator`)
//! disparaissait : de la signature, et des exclusions de références (son nom passait pour une référence).

use std::collections::HashMap;

use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};

const SRC: &str = "struct Foo { int x; };\n\nint f(Foo* p) {\n    return p->x;\n}\n\nint g(const Foo& r) {\n    return r.x;\n}\n\nint h(Foo&& m) {\n    return m.x;\n}\n";

#[test]
fn un_parametre_par_pointeur_ou_reference_garde_son_nom() {
    let chemin = "/virtual/a.cpp".to_string();
    let a = ProjectParser::new(ProjectParserOptions { verbose: false }).parse_project(ParseProjectOptions {
        root: "/virtual".to_string(),
        files: vec![chemin.clone()],
        content_map: Some(HashMap::from([(chemin, SRC.to_string())])),
        resolve_relationships: Some(false),
        resolver_options: None,
    });
    let scopes: Vec<_> = a.files.values().flat_map(|fa| fa.scopes.iter()).collect();
    for (fonction, nom) in [("f", "p"), ("g", "r"), ("h", "m")] {
        let s = scopes.iter().find(|s| s.name == fonction).unwrap_or_else(|| panic!("{fonction} est un scope"));
        let noms: Vec<&str> = s.parameters.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(noms, vec![nom], "{fonction} : ses paramètres ({})", s.signature);
        assert!(s.signature.contains(&format!("{nom})")), "{fonction} : la signature nomme {nom} ({})", s.signature);
        let refs: Vec<&str> = s.identifier_references.iter().map(|r| r.identifier.as_str()).collect();
        assert!(!refs.contains(&nom), "{fonction} : son paramètre {nom} n'est pas une référence ({refs:?})");
    }
}
