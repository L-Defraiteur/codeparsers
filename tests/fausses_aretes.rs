//! **Une relation que le code ne porte pas est un défaut, pas un réglage.**
//!
//! Chaque test reproduit, en petit, une fausse arête relevée le 3 octobre 2026
//! sur le corpus d'`e2e_code` (`src/dataflow/port.rs`, `services.rs`) ou sur
//! le moteur C++ (`src/include/binder/`) — voir l'état des lieux du même jour,
//! `extension/rag3weaver/docs/3-octobre-2026-20h30/`.
//!
//! Les cas tournent avec les options que rag3weaver passe au résolveur (ni
//! références de niveau fichier, ni références des enfants), et avec les
//! options par défaut quand le défaut en dépend.

use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};
use codeparsers::relationship_resolution::types::{RelationshipResolverOptions, RelationshipType, ResolvedRelationship};
use codeparsers::scope_extraction::types::UsageKind;
use std::collections::HashMap;

fn analyser(fichiers: &[(&str, &str)], options: Option<RelationshipResolverOptions>) -> Vec<ResolvedRelationship> {
    let racine = "/virtual";
    let mut contenus = HashMap::new();
    let mut chemins = Vec::new();
    for (nom, source) in fichiers {
        let chemin = format!("{racine}/{nom}");
        contenus.insert(chemin.clone(), source.to_string());
        chemins.push(chemin);
    }
    ProjectParser::new(ProjectParserOptions { verbose: false })
        .parse_project(ParseProjectOptions {
            root: racine.to_string(),
            files: chemins,
            content_map: Some(contenus),
            resolve_relationships: Some(true),
            resolver_options: options,
        })
        .relationships
        .map_or_else(Vec::new, |r| r.relationships)
}

/// Les deux jeux d'options : celui de rag3weaver, puis le défaut.
fn deux_modes(fichiers: &[(&str, &str)]) -> Vec<(&'static str, Vec<ResolvedRelationship>)> {
    vec![
        ("options de rag3weaver", analyser(fichiers, Some(RelationshipResolverOptions {
            include_file_level_refs: Some(false),
            include_child_refs: Some(false),
            ..Default::default()
        }))),
        ("options par défaut", analyser(fichiers, None)),
    ]
}

fn lister(rels: &[ResolvedRelationship]) -> Vec<String> {
    rels.iter().map(|r| format!("{:?} {} → {}", r.r#type, r.from_name, r.to_name)).collect()
}

fn est_heritage(t: &RelationshipType) -> bool {
    matches!(t, RelationshipType::INHERITSFROM | RelationshipType::IMPLEMENTS)
}

/// Une relation d'héritage dont aucun site n'est un site d'héritage est
/// fausse par construction : le type a été deviné sur le texte de la ligne.
fn aucun_heritage_sans_site_d_heritage(mode: &str, rels: &[ResolvedRelationship]) {
    let fausses: Vec<String> = rels.iter()
        .filter(|r| est_heritage(&r.r#type))
        .filter(|r| !r.metadata.as_ref().is_some_and(|m| m.sites.iter().any(|s| s.usage == UsageKind::Inheritance)))
        .map(|r| format!("{:?} {} → {} sites {:?}", r.r#type, r.from_name, r.to_name, r.metadata.as_ref().map(|m| &m.sites)))
        .collect();
    assert!(fausses.is_empty(), "[{mode}] héritages devinés sans site d'héritage : {fausses:#?}");
}

// ---------------------------------------------------------------------------
// 1. L'héritage deviné sur le texte
// ---------------------------------------------------------------------------

#[test]
fn rust_dans_un_impl_trait_seul_le_trait_est_implemente() {
    // services.rs : `impl Default for ServiceRegistry` faisait de toute
    // référence du scope une implémentation.
    let src = "trait Shape { fn area(&self) -> f64; }\nstruct Unit;\nfn helper() -> f64 { 1.0 }\nimpl Shape for Unit {\n    fn area(&self) -> f64 { helper() }\n}\n";
    for (mode, rels) in deux_modes(&[("unit.rs", src)]) {
        assert!(rels.iter().any(|r| r.r#type == RelationshipType::IMPLEMENTS && r.from_name == "Unit" && r.to_name == "Shape"),
            "[{mode}] la vraie implémentation reste : {:#?}", lister(&rels));
        assert!(!rels.iter().any(|r| est_heritage(&r.r#type) && r.to_name == "helper"),
            "[{mode}] appeler helper n'est pas l'implémenter : {:#?}", lister(&rels));
        aucun_heritage_sans_site_d_heritage(mode, &rels);
    }
}

#[test]
fn cpp_l_appel_du_constructeur_parent_n_est_pas_un_second_heritage() {
    // bound_use_database.h : `: BoundDatabaseStatement{…}` donnait un second
    // INHERITS_FROM, deviné sur le deux-points.
    let src = "class Base {\n public:\n  Base(int n);\n};\nclass Derived : public Base {\n public:\n  Derived() : Base{1} {}\n};\n";
    for (mode, rels) in deux_modes(&[("d.h", src)]) {
        let n = rels.iter().filter(|r| r.r#type == RelationshipType::INHERITSFROM && r.from_name == "Derived" && r.to_name == "Base").count();
        assert_eq!(n, 1, "[{mode}] un seul héritage Derived → Base : {:#?}", lister(&rels));
        aucun_heritage_sans_site_d_heritage(mode, &rels);
    }
}

#[test]
fn cpp_le_double_deux_points_n_est_pas_un_heritage() {
    let src = "class Base {};\nclass Maker {\n public:\n  Base* make();\n};\nBase* Maker::make() { return new Base(); }\n";
    for (mode, rels) in deux_modes(&[("m.cpp", src)]) {
        assert!(!rels.iter().any(|r| est_heritage(&r.r#type) && r.from_name == "make"),
            "[{mode}] `Maker::make` n'hérite de rien : {:#?}", lister(&rels));
        aucun_heritage_sans_site_d_heritage(mode, &rels);
    }
}

// ---------------------------------------------------------------------------
// 2. L'auto-implémentation
// ---------------------------------------------------------------------------

#[test]
fn rust_un_impl_ne_pointe_pas_vers_son_propre_type() {
    // port.rs : `impl PortValue` « implémentait » l'enum `PortValue`, parce
    // qu'une de ses méthodes écrit `PortValue::Trigger`.
    let src = "pub enum Value { A, B }\nimpl Value {\n    pub fn is_a(&self) -> bool {\n        matches!(self, Value::A)\n    }\n}\n";
    for (mode, rels) in deux_modes(&[("v.rs", src)]) {
        assert!(!rels.iter().any(|r| r.from_name == "Value" && r.to_name == "Value"
                && !matches!(r.r#type, RelationshipType::PARENTOF | RelationshipType::HASPARENT)),
            "[{mode}] un impl et son type sont la même chose : {:#?}", lister(&rels));
    }
}

// ---------------------------------------------------------------------------
// 3. La variable locale prise pour un symbole
// ---------------------------------------------------------------------------

#[test]
fn rust_une_variable_let_n_est_pas_la_methode_homonyme() {
    // port.rs : `let count = data.len();` reliait `new` à la méthode `count`.
    let src = "struct Batch { count: usize }\nimpl Batch {\n    fn count(&self) -> usize { self.count }\n    fn new(data: Vec<u8>) -> Self {\n        let count = data.len();\n        Self { count }\n    }\n}\n";
    for (mode, rels) in deux_modes(&[("b.rs", src)]) {
        assert!(!rels.iter().any(|r| r.from_name == "new" && r.to_name == "count"),
            "[{mode}] `count` est une variable de `new` : {:#?}", lister(&rels));
    }
}

#[test]
fn go_une_variable_courte_n_est_pas_la_fonction_homonyme() {
    let src = "package main\n\nfunc total() int { return 0 }\n\nfunc Use() int {\n\ttotal := 3\n\treturn total\n}\n";
    for (mode, rels) in deux_modes(&[("main.go", src)]) {
        assert!(!rels.iter().any(|r| r.from_name == "Use" && r.to_name == "total"),
            "[{mode}] `total` est une variable de `Use` : {:#?}", lister(&rels));
    }
}

// ---------------------------------------------------------------------------
// 4. Une relation par appel au lieu d'une par cible
// ---------------------------------------------------------------------------

#[test]
fn rust_trois_appels_font_une_relation_et_trois_sites() {
    // port.rs : huit `merge_port_values → take_or_clone`, un par ligne d'appel.
    let src = "fn a() {}\nfn run() {\n    a();\n    a();\n    a();\n}\n";
    for (mode, rels) in deux_modes(&[("r.rs", src)]) {
        let run_a: Vec<_> = rels.iter().filter(|r| r.r#type == RelationshipType::CONSUMES && r.from_name == "run" && r.to_name == "a").collect();
        assert_eq!(run_a.len(), 1, "[{mode}] une relation run → a : {:#?}", lister(&rels));
        let lignes: Vec<_> = run_a[0].metadata.as_ref().unwrap().sites.iter().map(|s| (s.usage.clone(), s.line)).collect();
        assert_eq!(lignes, vec![(UsageKind::Call, Some(3)), (UsageKind::Call, Some(4)), (UsageKind::Call, Some(5))], "[{mode}]");
        let inverses = rels.iter().filter(|r| r.r#type == RelationshipType::CONSUMEDBY && r.from_name == "a" && r.to_name == "run").count();
        assert_eq!(inverses, 1, "[{mode}] et un seul inverse");
    }
}

#[test]
fn python_un_appel_vu_deux_fois_fait_une_relation() {
    // test_backend_snapshot.py : chaque appel sortait comme appel et comme
    // identifiant, donc deux relations.
    let src = "def helper():\n    return 1\n\ndef run():\n    helper()\n";
    for (mode, rels) in deux_modes(&[("m.py", src)]) {
        let n = rels.iter().filter(|r| r.r#type == RelationshipType::CONSUMES && r.from_name == "run" && r.to_name == "helper").count();
        assert_eq!(n, 1, "[{mode}] une relation run → helper : {:#?}", lister(&rels));
    }
}

// ---------------------------------------------------------------------------
// 5. Un mot de commentaire pris pour un symbole
// ---------------------------------------------------------------------------

#[test]
fn un_mot_de_commentaire_n_est_pas_un_usage() {
    // services.rs:15 : « is » dans un commentaire de doc reliait la zone hors
    // scope à la méthode `is` de port.rs.
    let src = "//! The registry is shared.\n/// Needed because it is unsized.\nuse std::sync::Arc;\n\nfn is() -> bool { true }\n\nfn other() -> Arc<u8> { Arc::new(0) }\n";
    for (mode, rels) in deux_modes(&[("s.rs", src)]) {
        assert!(!rels.iter().any(|r| r.to_name == "is" && r.r#type == RelationshipType::CONSUMES),
            "[{mode}] un commentaire n'utilise rien : {:#?}", lister(&rels));
    }
}

#[test]
fn rust_un_appel_de_methode_sur_une_variable_locale_reste_une_reference() {
    // Exclure les variables `let` cache l'identifiant nu, pas le membre :
    // `node.with_delai(t)` (run_nodes.rs) reste une référence à `with_delai`.
    // Qu'elle devienne une relation dépend du résolveur (le qualificatif doit
    // être un scope connu), pas de ce lot.
    let chemin = "/virtual/n.rs".to_string();
    let src = "struct Node;\nimpl Node {\n    fn with_delai(self, t: u64) -> Self { self }\n}\nfn build(t: u64) -> Node {\n    let mut node = Node;\n    node = node.with_delai(t);\n    node\n}\n";
    let analyse = ProjectParser::new(ProjectParserOptions { verbose: false }).parse_project(ParseProjectOptions {
        root: "/virtual".to_string(),
        files: vec![chemin.clone()],
        content_map: Some(HashMap::from([(chemin.clone(), src.to_string())])),
        resolve_relationships: Some(false),
        resolver_options: None,
    });
    let build = analyse.files[&chemin].scopes.iter().find(|s| s.name == "build").expect("le scope build");
    let noms: Vec<(String, Option<String>)> = build.identifier_references.iter().map(|r| (r.identifier.clone(), r.qualifier.clone())).collect();
    assert!(noms.contains(&("with_delai".to_string(), Some("node".to_string()))), "références de build : {noms:?}");
    assert!(!noms.iter().any(|(n, q)| n == "node" && q.is_none()), "`node` nu est une variable locale : {noms:?}");
}

#[test]
fn rust_l_initialiseur_d_un_let_masquant_appelle_encore_la_fonction() {
    // validation_nodes.rs : `let message = message(…)` appelle la fonction
    // `message`, puis lie une variable du même nom.
    let src = "fn message(x: u32) -> u32 { x }\nfn rule() -> u32 {\n    let message = message(1);\n    message\n}\n";
    for (mode, rels) in deux_modes(&[("v.rs", src)]) {
        let r = rels.iter().find(|r| r.r#type == RelationshipType::CONSUMES && r.from_name == "rule" && r.to_name == "message");
        let sites = r.and_then(|r| r.metadata.as_ref()).map(|m| m.sites.iter().map(|s| (s.usage.clone(), s.line)).collect::<Vec<_>>());
        assert_eq!(sites, Some(vec![(UsageKind::Call, Some(3))]), "[{mode}] l'appel seul, pas la variable : {:#?}", lister(&rels));
    }
}
