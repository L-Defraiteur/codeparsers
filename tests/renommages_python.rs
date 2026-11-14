//! **Les distributions Python dont le module porte un autre nom** (14 novembre 2026, lot 1 de la
//! relecture des règles) : `PyYAML` s'importe `yaml`, `scikit-learn` `sklearn`. La table livrée
//! (`data/imports/python.toml`, `[distributions]`, tirée de pipreqs) fait le lien ; le dépôt la complète
//! ou la remplace par `import_renames` (le champ `imports` du manifeste du backend).

use codeparsers::import_resolution::provenance::Provenance;
use codeparsers::import_resolution::regles::Regles;
use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};
use codeparsers::relationship_resolution::types::{ImportRenames, RelationshipResolverOptions};
use std::collections::HashMap;

const SOURCE: &str = "import yaml\nimport sklearn\nimport mon_module\n\ndef lire():\n    return yaml.load, sklearn.base, mon_module.f\n";

fn provenances(renames: Option<HashMap<String, ImportRenames>>) -> HashMap<String, Option<Provenance>> {
    let chemin = "/virtual/a.py".to_string();
    let python: Vec<String> = ["PyYAML", "scikit-learn", "mon-paquet"].iter().map(|s| s.to_string()).collect();
    let r = ProjectParser::new(ProjectParserOptions { verbose: false })
        .parse_project(ParseProjectOptions {
            root: "/virtual".to_string(),
            files: vec![chemin.clone()],
            content_map: Some(HashMap::from([(chemin.clone(), SOURCE.to_string())])),
            resolve_relationships: Some(true),
            resolver_options: Some(RelationshipResolverOptions {
                include_file_level_refs: Some(false),
                include_child_refs: Some(false),
                resolve_cross_file: Some(false),
                project_files: Some(vec![chemin.clone()]),
                declared_dependencies_by_language: Some(HashMap::from([("python".to_string(), python)])),
                import_renames: renames,
                ..Default::default()
            }),
        })
        .relationships
        .expect("une résolution");
    r.external_libraries.values().map(|l| (l.name.clone(), l.provenance)).collect()
}

fn du_depot(replace: bool) -> Option<HashMap<String, ImportRenames>> {
    Some(HashMap::from([(
        "python".to_string(),
        ImportRenames { replace, distributions: HashMap::from([("mon-paquet".to_string(), vec!["mon_module".to_string()])]) },
    )]))
}

#[test]
fn la_table_livree_relie_pyyaml_a_yaml() {
    let p = provenances(None);
    assert_eq!(p.get("yaml"), Some(&Some(Provenance::Dependency)), "PyYAML s'importe yaml : {p:?}");
    assert_eq!(p.get("sklearn"), Some(&Some(Provenance::Dependency)), "scikit-learn s'importe sklearn : {p:?}");
    assert_eq!(p.get("mon_module"), Some(&Some(Provenance::Unknown)), "sans la table du dépôt, rien ne le relie : {p:?}");
}

#[test]
fn le_depot_complete_la_table_ou_la_remplace() {
    let p = provenances(du_depot(false));
    assert_eq!(p.get("mon_module"), Some(&Some(Provenance::Dependency)), "{p:?}");
    assert_eq!(p.get("yaml"), Some(&Some(Provenance::Dependency)), "la table livrée tient encore : {p:?}");
    let p = provenances(du_depot(true));
    assert_eq!(p.get("mon_module"), Some(&Some(Provenance::Dependency)), "{p:?}");
    assert_eq!(p.get("yaml"), Some(&Some(Provenance::Unknown)), "replace : la table livrée ne compte plus : {p:?}");
}

#[test]
fn les_langues_qui_ont_des_regles() {
    assert_eq!(Regles::charger(None).langues(), vec!["c", "cpp", "csharp", "go", "javascript", "python", "rust"]);
}
