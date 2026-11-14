//! **Les alias de résolution** (14 novembre 2026, point 4 de la relecture des règles) : un import qui
//! désigne un fichier du dépôt par un alias n'est pas une dépendance. `#…` (les imports internes de
//! Node, champ `imports` de package.json) l'est toujours : c'est une donnée de la langue
//! (`javascript.toml`, `[local]`). Les alias d'un dépôt (`paths` de tsconfig : `~/`, `@app/`) se
//! déclarent par `local_prefixes_by_language`.

use codeparsers::import_resolution::provenance::Provenance;
use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};
use codeparsers::relationship_resolution::types::RelationshipResolverOptions;
use std::collections::HashMap;

const SOURCE: &str = "import { a } from \"#utils/a\";\nimport { b } from \"~/lib/b\";\nimport { c } from \"@app/c\";\nimport _ from \"lodash\";\n\nexport function lancer() {\n  return [a, b, c, _.head];\n}\n";

fn provenances(prefixes: Option<Vec<&str>>) -> HashMap<String, Option<Provenance>> {
    let chemin = "/virtual/src/main.ts".to_string();
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
                local_prefixes_by_language: prefixes.map(|p| HashMap::from([("javascript".to_string(), p.iter().map(|s| s.to_string()).collect())])),
                ..Default::default()
            }),
        })
        .relationships
        .expect("une résolution");
    r.external_libraries.values().map(|l| (l.name.clone(), l.provenance)).collect()
}

#[test]
fn un_import_interne_de_node_est_du_projet() {
    let p = provenances(None);
    assert_eq!(p.get("#utils/a"), Some(&Some(Provenance::Project)), "{p:?}");
    assert_eq!(p.get("~/lib/b"), Some(&Some(Provenance::Unknown)), "sans alias déclaré, rien ne le relie : {p:?}");
    assert_eq!(p.get("lodash"), Some(&Some(Provenance::Unknown)), "{p:?}");
}

#[test]
fn un_alias_declare_du_depot_est_du_projet() {
    let p = provenances(Some(vec!["~/", "@app/"]));
    assert_eq!(p.get("~/lib/b"), Some(&Some(Provenance::Project)), "{p:?}");
    assert_eq!(p.get("@app/c"), Some(&Some(Provenance::Project)), "{p:?}");
    assert_eq!(p.get("lodash"), Some(&Some(Provenance::Unknown)), "un alias ne touche pas un paquet : {p:?}");
}
