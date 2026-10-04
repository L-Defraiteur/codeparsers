//! **Une référence appartient au scope le plus étroit qui la contient.** Un
//! conteneur — `mod tests` en Rust, `namespace` en C++ — portait aussi les
//! références de tous ses membres, que ses membres portent déjà :
//! `tests → name` reliait le module de tests de graph_tool.rs à la méthode
//! `GraphTool::name`, parce que des fonctions de test y ont un paramètre
//! nommé `name`. Le conteneur ne garde que ce qui est à lui.

use std::collections::HashMap;

use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};

fn refs(nom_fichier: &str, source: &str, scope: &str) -> Vec<String> {
    let chemin = format!("/virtual/{nom_fichier}");
    let a = ProjectParser::new(ProjectParserOptions { verbose: false }).parse_project(ParseProjectOptions {
        root: "/virtual".to_string(),
        files: vec![chemin.clone()],
        content_map: Some(HashMap::from([(chemin, source.to_string())])),
        resolve_relationships: Some(false),
        resolver_options: None,
    });
    a.files.values().flat_map(|f| f.scopes.iter().filter(|s| s.name == scope).flat_map(|s| s.identifier_references.iter().map(|r| r.identifier.clone()))).collect()
}

#[test]
fn rust_un_mod_tests_ne_porte_pas_les_references_de_ses_tests() {
    let src = "pub fn helper() -> u32 {\n    1\n}\n\n#[cfg(test)]\nmod tests {\n    use super::helper;\n\n    fn find(name: &str) -> bool {\n        name.is_empty()\n    }\n\n    #[test]\n    fn marche() {\n        assert_eq!(helper(), 1);\n        find(\"x\");\n    }\n}\n";
    let module = refs("lib.rs", src, "tests");
    for nom in ["name", "is_empty", "find"] {
        assert!(!module.contains(&nom.to_string()), "« {nom} » est à un membre de tests, pas au module : {module:?}");
    }
    // Les membres gardent les leurs.
    assert!(refs("lib.rs", src, "marche").contains(&"helper".to_string()));
    assert!(refs("lib.rs", src, "find").contains(&"is_empty".to_string()));
}

#[test]
fn cpp_un_namespace_ne_porte_pas_les_references_de_ses_fonctions() {
    let src = "namespace kz {\nint aide(int a);\nint libre(int a) {\n    return aide(a) + 1;\n}\n}\n";
    let ns = refs("f.cpp", src, "kz");
    assert!(!ns.contains(&"aide".to_string()), "l'appel est dans libre, pas dans le namespace : {ns:?}");
    assert!(refs("f.cpp", src, "libre").contains(&"aide".to_string()));
}
