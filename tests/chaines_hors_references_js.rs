//! **Un mot dans une chaîne n'est pas une référence** (13 novembre 2026, ticket rag3db
//! `docs/codeparsers/tickets/2026-11-13-un-mot-dans-une-chaine-devient-une-reference-d-une-variable-js.md`).
//! `ensure_import_references_tracked` cherche les noms importés dans le texte d'un scope, chaînes
//! effacées ; l'apostrophe n'y fermait qu'un caractère seul (`'x'`, la règle de Rust), et une chaîne
//! JavaScript `'path'` restait du code : la variable `ES_BUILD_CONFIG` et la fonction `lancer`
//! recevaient une référence `path`, puis une arête `liaison` vers le `require("path")`.

use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};
use codeparsers::relationship_resolution::types::RelationshipResolverOptions;
use codeparsers::scope_extraction::types::ScopeInfo;
use std::collections::HashMap;

fn scopes(nom: &str, source: &str) -> (Vec<ScopeInfo>, Vec<(String, String)>) {
    let chemin = format!("/virtual/{nom}");
    let a = ProjectParser::new(ProjectParserOptions { verbose: false }).parse_project(ParseProjectOptions {
        root: "/virtual".into(),
        files: vec![chemin.clone()],
        content_map: Some(HashMap::from([(chemin, source.to_string())])),
        resolve_relationships: Some(true),
        resolver_options: Some(RelationshipResolverOptions {
            include_file_level_refs: Some(false),
            include_child_refs: Some(false),
            resolve_cross_file: Some(false),
            ..Default::default()
        }),
    });
    // Les CONSUMES seules : la réciproque CONSUMED_BY (`lancer` consommé par `module.exports`) est légitime.
    let rels = a
        .relationships
        .map(|r| r.relationships.iter().filter(|x| format!("{:?}", x.r#type) == "CONSUMES").map(|x| (x.from_name.clone(), x.to_name.clone())).collect())
        .unwrap_or_default();
    (a.files.values().flat_map(|f| f.scopes.clone()).collect(), rels)
}

/// Les lignes où un scope nomme `nom`, comme identifiant ou comme qualificatif (`path.join` : `join`
/// qualifié par `path`).
fn refs(scopes: &[ScopeInfo], scope: &str, nom: &str) -> Vec<usize> {
    scopes.iter().filter(|s| s.name == scope).flat_map(|s| s.identifier_references.iter().filter(|r| r.identifier == nom || r.qualifier.as_deref() == Some(nom)).map(|r| r.line)).collect()
}

const BUILD: &str = r#"const path = require("path");

const ES_BUILD_CONFIG = {
  external: ['fs', 'path'],
};

async function lancer(page) {
  const firstPath = await page.locator('.result-path').first();
  if (firstPath.startsWith('doc')) throw new Error('Expected file path, got: ' + firstPath);
  return ES_BUILD_CONFIG;
}

function joindre(a, b) {
  return path.join(a, b);
}

module.exports = { lancer, joindre };
"#;

#[test]
fn un_mot_dans_une_chaine_js_n_est_pas_une_reference() {
    let (s, rels) = scopes("build.js", BUILD);
    assert_eq!(refs(&s, "ES_BUILD_CONFIG", "path"), Vec::<usize>::new(), "['fs', 'path'] ne nomme pas l'import");
    assert_eq!(refs(&s, "lancer", "path"), Vec::<usize>::new(), "'.result-path' ne nomme pas l'import");
    // Aucune arête vers la zone de la ligne `require("path")` (la liaison) depuis eux ; `lancer` garde
    // la sienne vers `ES_BUILD_CONFIG`, qu'il renvoie.
    assert!(
        !rels.iter().any(|(de, vers)| (de == "ES_BUILD_CONFIG" || de == "lancer") && vers.starts_with("file_scope")),
        "aucune arête depuis eux vers la liaison : {rels:?}"
    );
    // Le témoin positif : un vrai usage reste une référence.
    assert!(!refs(&s, "joindre", "path").is_empty(), "path.join(a, b) est un usage");
}

/// Python : les chaînes entre apostrophes sont des chaînes aussi.
#[test]
fn un_mot_dans_une_chaine_python_n_est_pas_une_reference() {
    let source = "import os\n\ndef config():\n    return {'external': ['os', 'sys']}\n\ndef lire():\n    return os.getcwd()\n";
    let (s, _) = scopes("conf.py", source);
    assert_eq!(refs(&s, "config", "os"), Vec::<usize>::new(), "['os', 'sys'] ne nomme pas l'import");
    assert!(!refs(&s, "lire", "os").is_empty(), "os.getcwd() est un usage");
}
