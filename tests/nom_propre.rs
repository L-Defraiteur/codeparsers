//! **Le nom du scope n'est pas un mot interdit dans son corps.** Exclu par
//! son texte, il cachait toute occurrence du nom : l'appel récursif, et
//! l'appel d'une méthode homonyme sur un autre receveur. Deux appels de
//! `QueryProcessor::acquireLocksBeforeExecution` manquaient ainsi sur trois
//! (processor.cpp:69 et :71, retours du cœur, 11 octobre 2026). Seul le nœud
//! qui déclare le nom s'écarte, par sa position.

use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};
use std::collections::HashMap;

/// Les références (nom, ligne, qualificatif) du scope `scope` (de parent
/// `parent` s'il est donné).
fn refs(fichier: &str, src: &str, scope: &str, parent: Option<&str>) -> Vec<(String, usize, Option<String>)> {
    let chemin = format!("/virtual/{fichier}");
    let analyse = ProjectParser::new(ProjectParserOptions { verbose: false }).parse_project(ParseProjectOptions {
        root: "/virtual".to_string(),
        files: vec![chemin.clone()],
        content_map: Some(HashMap::from([(chemin.clone(), src.to_string())])),
        resolve_relationships: Some(false),
        resolver_options: None,
    });
    let sc = analyse.files[&chemin]
        .scopes
        .iter()
        .find(|s| s.name == scope && (parent.is_none() || s.parent.as_deref() == parent))
        .unwrap_or_else(|| panic!("le scope {scope} ({parent:?})"));
    sc.identifier_references.iter().map(|r| (r.identifier.clone(), r.line, r.qualifier.clone())).collect()
}

fn a(r: &[(String, usize, Option<String>)], nom: &str, ligne: usize) -> bool {
    r.iter().any(|(n, l, _)| n == nom && *l == ligne)
}

#[test]
fn cpp_l_appel_recursif_et_la_methode_homonyme_d_un_autre_receveur_sont_des_references() {
    let src = "struct Task;\nstruct Op { void run(int c); };\nstruct Proc {\n    Op* sink;\n    void run(Task* t, int c);\n};\nvoid Proc::run(Task* t, int c) {\n    run(t, c);\n    sink->run(c);\n}\n";
    let r = refs("p.cpp", src, "run", Some("Proc"));
    assert!(a(&r, "run", 8), "l'appel récursif : {r:?}");
    assert!(r.iter().any(|(n, l, q)| n == "run" && *l == 9 && q.as_deref() == Some("sink")), "la méthode homonyme de sink : {r:?}");
    assert!(!a(&r, "run", 7) && !a(&r, "Proc", 7), "la déclaration `Proc::run` n'est pas un usage : {r:?}");
}

#[test]
fn rust_une_methode_qui_appelle_son_homonyme_d_un_autre_type_le_reference() {
    // (`new` est un mot vide de la liste commune : un autre nom ici.)
    let src = "struct Inner;\nimpl Inner {\n    fn build() -> Self { Inner }\n}\nstruct Outer { i: Inner }\nimpl Outer {\n    fn build() -> Self {\n        Outer { i: Inner::build() }\n    }\n}\nfn fact(n: u64) -> u64 {\n    if n == 0 { 1 } else { n * fact(n - 1) }\n}\n";
    let r = refs("o.rs", src, "build", Some("Outer"));
    assert!(r.iter().any(|(n, l, q)| n == "build" && *l == 8 && q.as_deref() == Some("Inner")), "Inner::build dans Outer::build : {r:?}");
    assert!(!a(&r, "build", 7), "la déclaration de build n'est pas un usage : {r:?}");
    let r = refs("o.rs", src, "fact", None);
    assert!(a(&r, "fact", 12), "l'appel récursif : {r:?}");
    assert!(!a(&r, "fact", 11), "la déclaration : {r:?}");
}

#[test]
fn python_l_appel_recursif_est_une_reference() {
    let src = "def fact(n):\n    return 1 if n == 0 else n * fact(n - 1)\n";
    let r = refs("f.py", src, "fact", None);
    assert!(a(&r, "fact", 2), "l'appel récursif : {r:?}");
    assert!(!a(&r, "fact", 1), "la déclaration : {r:?}");
}

#[test]
fn go_l_appel_recursif_est_une_reference() {
    let src = "package p\n\nfunc fact(n int) int {\n\tif n == 0 {\n\t\treturn 1\n\t}\n\treturn n * fact(n-1)\n}\n";
    let r = refs("f.go", src, "fact", None);
    assert!(a(&r, "fact", 7), "l'appel récursif : {r:?}");
    assert!(!a(&r, "fact", 3), "la déclaration : {r:?}");
}
