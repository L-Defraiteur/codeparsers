//! **Les règles d'import en données** (14 novembre 2026, conception « du nom importé au paquet ») : la
//! liste de la bibliothèque standard et la règle du nom de paquet de Python viennent de
//! `data/imports/python.toml`, compilé comme défaut ; un fichier du même nom dans `import_rules_dir`
//! remplace **la langue entière**, et l'origine des règles se dit ; un fichier illisible retombe sur le
//! défaut en le disant.

use codeparsers::import_resolution::provenance::{est_builtin, est_builtin_selon, nom_de_paquet_selon, Provenance};
use codeparsers::import_resolution::regles::Regles;
use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};
use codeparsers::relationship_resolution::types::RelationshipResolverOptions;
use std::collections::HashMap;

/// Un dossier de règles neuf sous le target du test, avec `python.toml` = `texte`.
fn dossier(nom: &str, texte: &str) -> String {
    dossier_de(nom, "python", texte)
}

/// Un dossier de règles neuf sous le target du test, avec `<langue>.toml` = `texte`.
fn dossier_de(nom: &str, langue: &str, texte: &str) -> String {
    let d = std::path::Path::new(env!("CARGO_TARGET_TMPDIR")).join("regles_imports").join(nom);
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join(format!("{langue}.toml")), texte).unwrap();
    d.display().to_string()
}

const REMPLACEE: &str = r#"
langue = "python"
extensions = ["py", "pyi"]

[builtin]
source = "témoin"
noms = ["requests"]

[nom_de_paquet]
separateur = "."
garder = 1
"#;

#[test]
fn les_regles_livrees_tiennent_la_bibliotheque_standard() {
    let r = Regles::charger(None);
    assert_eq!(r.origines.get("python").map(String::as_str), Some("défaut"));
    assert!(est_builtin_selon(&r, "a.py", "os"));
    assert!(est_builtin_selon(&r, "a.py", "asyncio.tasks"));
    assert!(est_builtin_selon(&r, "a.pyi", "tomllib"));
    assert!(!est_builtin_selon(&r, "a.py", "requests"));
    assert_eq!(nom_de_paquet_selon(&r, "a.py", "requests.adapters"), "requests");
    assert_eq!(nom_de_paquet_selon(&r, "a.py", "PyYAML"), "pyyaml");
    assert_eq!(nom_de_paquet_selon(&r, "a.py", "my-pkg.sub"), "my_pkg");
}

#[test]
fn un_dossier_remplace_la_langue_entiere() {
    let d = dossier("remplace", REMPLACEE);
    let r = Regles::charger(Some(&d));
    assert!(est_builtin_selon(&r, "a.py", "requests.adapters"), "le nom du fichier du dépôt");
    assert!(!est_builtin_selon(&r, "a.py", "os"), "la liste livrée ne se fusionne pas");
    assert!(r.origines["python"].ends_with("python.toml"), "{:?}", r.origines);
    // Sans variantes dans le fichier, la casse ne se confond plus.
    assert_eq!(nom_de_paquet_selon(&r, "a.py", "PyYAML.x"), "PyYAML");
}

#[test]
fn un_fichier_illisible_retombe_sur_le_defaut_en_le_disant() {
    let d = dossier("illisible", "langue = 3\n");
    let r = Regles::charger(Some(&d));
    assert!(est_builtin_selon(&r, "a.py", "os"));
    assert!(r.origines["python"].starts_with("défaut (") && r.origines["python"].contains("refusé"), "{:?}", r.origines);
}

#[test]
fn le_resolveur_lit_le_dossier_du_depot() {
    let d = dossier("resolveur", REMPLACEE);
    let source = "import os\nimport requests\n\ndef lire():\n    return os.getcwd(), requests.get\n";
    let chemin = "/virtual/a.py".to_string();
    let provenance = |dossier: Option<String>, nom: &str| {
        let r = ProjectParser::new(ProjectParserOptions { verbose: false })
            .parse_project(ParseProjectOptions {
                root: "/virtual".to_string(),
                files: vec![chemin.clone()],
                content_map: Some(HashMap::from([(chemin.clone(), source.to_string())])),
                resolve_relationships: Some(true),
                resolver_options: Some(RelationshipResolverOptions {
                    include_file_level_refs: Some(false),
                    include_child_refs: Some(false),
                    resolve_cross_file: Some(false),
                    project_files: Some(vec![chemin.clone()]),
                    import_rules_dir: dossier,
                    ..Default::default()
                }),
            })
            .relationships
            .expect("une résolution");
        r.external_libraries.values().find(|l| l.name == nom).and_then(|l| l.provenance)
    };
    assert_eq!(provenance(None, "requests"), Some(Provenance::Unknown));
    assert_eq!(provenance(None, "os"), Some(Provenance::Builtin));
    assert_eq!(provenance(Some(d.clone()), "requests"), Some(Provenance::Builtin));
    assert_eq!(provenance(Some(d), "os"), Some(Provenance::Unknown));
}

#[test]
fn javascript_la_liste_de_node_et_le_scope() {
    let r = Regles::charger(None);
    assert_eq!(r.origines.get("javascript").map(String::as_str), Some("défaut"), "{:?}", r.origines);
    // `async_hooks`, `diagnostics_channel` et `http2` manquaient à l'ancienne liste du code.
    for nom in ["fs", "fs/promises", "async_hooks", "diagnostics_channel", "http2", "node:sqlite", "node:test"] {
        assert!(est_builtin("a.ts", nom), "{nom} est du moteur");
    }
    assert!(!est_builtin("a.js", "lodash/fp"));
    assert_eq!(nom_de_paquet_selon(&r, "a.tsx", "@Scope/my-pkg/sub"), "@scope/my_pkg");
    assert_eq!(nom_de_paquet_selon(&r, "a.mjs", "lodash/fp"), "lodash");
}

#[test]
fn javascript_un_dossier_remplace_la_langue_sans_toucher_python() {
    let d = dossier_de("javascript", "javascript", "langue = \"javascript\"\nextensions = [\"js\", \"ts\"]\n\n[builtin]\nnoms = [\"electron\"]\n");
    let r = Regles::charger(Some(&d));
    assert!(est_builtin_selon(&r, "a.ts", "electron"));
    assert!(!est_builtin_selon(&r, "a.js", "fs"), "la liste de Node ne se fusionne pas");
    assert!(est_builtin_selon(&r, "a.py", "os"), "python garde ses règles livrées");
    assert_eq!(r.origines["python"], "défaut");
}

#[test]
fn rust_la_crate_d_un_chemin_absolu_et_le_tiret() {
    let r = Regles::charger(None);
    assert_eq!(r.origines.get("rust").map(String::as_str), Some("défaut"), "{:?}", r.origines);
    assert!(est_builtin("a.rs", "std::fs"));
    // `::std::fs` (chemin absolu) : l'ancien code y voyait une crate vide.
    assert!(est_builtin("a.rs", "::std::fs"));
    assert!(!est_builtin("a.rs", "serde::Serialize"));
    assert_eq!(nom_de_paquet_selon(&r, "a.rs", "::serde_json::Value"), "serde_json");
    assert_eq!(nom_de_paquet_selon(&r, "a.rs", "Serde_Json::Value"), "serde_json");
    // Cargo dit `serde-json`, l'import `serde_json` : les deux se normalisent pareil.
    assert_eq!(r.de_fichier("a.rs").unwrap().normaliser("serde-json"), "serde_json");
}
