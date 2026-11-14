//! **Un import Python pointé** (14 novembre 2026, ticket rag3db
//! `docs/codeparsers/tickets/2026-11-14-un-import-python-pointe-ne-produit-aucune-bibliotheque.md`) :
//! `import requests.adapters` lie le nom `requests` dans le fichier ; une mention de `requests` est un
//! usage de la bibliothèque. Les formes relatives (`from . import x`, `from .outils import f`) restent du
//! projet, sans bibliothèque.

use codeparsers::import_resolution::provenance::Provenance;
use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};
use codeparsers::relationship_resolution::types::{RelationshipResolutionResult, RelationshipResolverOptions};
use std::collections::HashMap;

fn resoudre(fichiers: &[(&str, &str)]) -> RelationshipResolutionResult {
    let mut contenus = HashMap::new();
    let mut chemins = Vec::new();
    for (nom, source) in fichiers {
        let chemin = format!("/virtual/{nom}");
        contenus.insert(chemin.clone(), source.to_string());
        chemins.push(chemin);
    }
    ProjectParser::new(ProjectParserOptions { verbose: false })
        .parse_project(ParseProjectOptions {
            root: "/virtual".to_string(),
            files: chemins.clone(),
            content_map: Some(contenus),
            resolve_relationships: Some(true),
            resolver_options: Some(RelationshipResolverOptions {
                include_file_level_refs: Some(false),
                include_child_refs: Some(false),
                resolve_cross_file: Some(false),
                project_files: Some(chemins),
                declared_dependencies: Some(vec!["requests".to_string()]),
                ..Default::default()
            }),
        })
        .relationships
        .expect("une résolution")
}

#[test]
fn un_import_pointe_produit_sa_bibliotheque() {
    let source = "import requests.adapters\nimport os.path\n\ndef lire():\n    return requests.adapters.HTTPAdapter, os.path.join('a', 'b')\n";
    let r = resoudre(&[("main.py", source)]);
    let noms: Vec<_> = r.external_libraries.values().map(|l| (l.name.clone(), l.provenance)).collect();
    let req = r.external_libraries.values().find(|l| l.name.starts_with("requests")).map(|l| l.provenance);
    assert_eq!(req, Some(Some(Provenance::Dependency)), "import requests.adapters : la bibliothèque requests, dépendance déclarée : {noms:?}");
    let os = r.external_libraries.values().find(|l| l.name.starts_with("os")).map(|l| l.provenance);
    assert_eq!(os, Some(Some(Provenance::Builtin)), "import os.path : la bibliothèque standard : {noms:?}");
    let usages: Vec<_> = r.relationships.iter().filter(|x| format!("{:?}", x.r#type) == "USESLIBRARY" && x.from_name == "lire").map(|x| x.to_name.clone()).collect();
    assert!(usages.iter().any(|n| n.starts_with("requests")), "lire utilise requests : {usages:?}");
    assert!(usages.iter().any(|n| n.starts_with("os")), "lire utilise os : {usages:?}");
}

#[test]
fn un_import_relatif_reste_du_projet() {
    let main = "from . import outils\nfrom .outils import f\n\ndef lire():\n    return outils.f(), f()\n";
    let r = resoudre(&[("paquet/main.py", main), ("paquet/outils.py", "def f():\n    return 1\n"), ("paquet/__init__.py", "")]);
    let noms: Vec<_> = r.external_libraries.values().map(|l| l.name.clone()).collect();
    assert!(noms.is_empty(), "un import relatif n'est pas une bibliothèque : {noms:?}");
}

/// La forme exacte du ticket : la mention est `requests.adapters` seule (un module, pas un attribut).
#[test]
fn la_forme_du_ticket_une_mention_du_sous_module_seul() {
    let source = "import os\nimport requests.adapters\nimport yaml\n\ndef lire():\n    return os.getcwd(), requests.adapters, yaml.load\n";
    let r = resoudre(&[("main.py", source)]);
    let noms: Vec<_> = r.external_libraries.values().map(|l| (l.name.clone(), l.provenance)).collect();
    assert!(r.external_libraries.values().any(|l| l.name.starts_with("requests")), "import requests.adapters : sa bibliothèque : {noms:?}");
}
