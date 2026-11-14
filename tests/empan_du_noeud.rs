//! **L'empan du nœud d'un scope** (14 novembre 2026, à la demande de rag3db) : `scope_start_byte` et
//! `scope_end_byte` sont à la ligne (le contrat de `File`). Dans un bundle minifié d'une seule ligne,
//! chaque fonction recevait ainsi la ligne entière : la mesure de lucivy a trouvé un même morceau de
//! 133 Ko répété 484 fois. `node_start_byte` et `node_end_byte` disent où le scope commence et finit
//! **dans** ses lignes.

use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};
use codeparsers::scope_extraction::types::ScopeInfo;
use std::collections::HashMap;

fn scopes(nom: &str, source: &str) -> Vec<ScopeInfo> {
    let chemin = format!("/virtual/{nom}");
    let a = ProjectParser::new(ProjectParserOptions { verbose: false }).parse_project(ParseProjectOptions {
        root: "/virtual".into(),
        files: vec![chemin.clone()],
        content_map: Some(HashMap::from([(chemin, source.to_string())])),
        resolve_relationships: Some(false),
        resolver_options: None,
    });
    a.files.values().flat_map(|f| f.scopes.clone()).collect()
}

fn texte_du_noeud<'a>(source: &'a str, s: &ScopeInfo) -> &'a str {
    let (d, f) = (s.node_start_byte.expect("un scope né d'un nœud a son empan"), s.node_end_byte.expect("et sa fin"));
    &source[d..f]
}

/// Deux fonctions sur une seule ligne : chacune n'a que ses octets, et les deux empans sont
/// disjoints ; les octets à la ligne restent ceux de la ligne entière.
#[test]
fn deux_fonctions_sur_une_ligne_ont_chacune_leur_empan() {
    let source = "function a(){return 1}function b(){return 2}\n";
    let s = scopes("bundle.min.js", source);
    let a = s.iter().find(|x| x.name == "a").expect("la fonction a");
    let b = s.iter().find(|x| x.name == "b").expect("la fonction b");
    assert_eq!(texte_du_noeud(source, a), "function a(){return 1}");
    assert_eq!(texte_du_noeud(source, b), "function b(){return 2}");
    assert!(a.node_end_byte.unwrap() <= b.node_start_byte.unwrap(), "empans disjoints");
    assert_eq!((a.scope_start_byte, b.scope_start_byte), (0, 0), "les octets à la ligne ne changent pas");
}

/// Un scope qui occupe ses lignes entières : l'empan du nœud est le texte de ces lignes, sans le
/// saut de ligne final ni l'indentation de tête.
#[test]
fn un_scope_de_lignes_entieres_garde_son_texte() {
    let source = "fn un() -> u32 {\n    1\n}\n\nfn deux() -> u32 {\n    2\n}\n";
    let s = scopes("lib.rs", source);
    let deux = s.iter().find(|x| x.name == "deux").expect("la fonction deux");
    assert_eq!(texte_du_noeud(source, deux), "fn deux() -> u32 {\n    2\n}");
    assert_eq!(&source[deux.scope_start_byte..deux.scope_end_byte], "fn deux() -> u32 {\n    2\n}");
}

/// Un trou entre déclarations n'a pas de nœud : son empan est `None`, ses octets à la ligne restent.
#[test]
fn un_trou_entre_declarations_n_a_pas_d_empan_de_noeud() {
    let source = "// en-tête\nconst x = 1;\nfunction f() { return x }\n";
    let s = scopes("trou.js", source);
    for t in s.iter().filter(|x| x.name.starts_with("file_scope")) {
        assert!(t.node_start_byte.is_none() && t.node_end_byte.is_none(), "{}", t.name);
    }
}
