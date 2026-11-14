//! **Les imports Go produisent leurs bibliothèques** (14 novembre 2026, ticket « un import Go ne produit
//! aucune bibliothèque ») : le parseur Go lisait ses imports avec les expressions de JavaScript
//! (`import X from '…'`) et n'en trouvait aucun. Ils se lisent maintenant sur l'arbre (`import_spec` :
//! `path`, `name`) ; le nom lié est l'alias, sinon le dernier élément du chemin (`/v2` renvoie à
//! l'élément d'avant, `yaml.v3` donne `yaml`). Leur provenance vient de `go.mod`, casse gardée.

use codeparsers::import_resolution::provenance::Provenance;
use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};
use codeparsers::relationship_resolution::types::{RelationshipResolutionResult, RelationshipResolverOptions};
use std::collections::HashMap;

const SOURCE: &str = "package main\n\nimport (\n\t\"fmt\"\n\t\"github.com/Azure/azure-sdk-for-go/sdk/azcore/policy\"\n\taz \"github.com/azure/azure-sdk-for-go/sdk/azcore\"\n\t\"gopkg.in/yaml.v3\"\n\t_ \"github.com/lib/pq\"\n\t\"github.com/x/mod/v2\"\n)\n\nfunc lancer() {\n\tfmt.Println(\"a\")\n\tpolicy.Lancer()\n\taz.Autre()\n\tyaml.Unmarshal(nil, nil)\n\tmod.F()\n}\n";

fn resoudre() -> RelationshipResolutionResult {
    let chemin = "/virtual/main.go".to_string();
    let go: Vec<String> = ["github.com/Azure/azure-sdk-for-go/sdk/azcore", "gopkg.in/yaml.v3", "github.com/x/mod/v2"].iter().map(|s| s.to_string()).collect();
    ProjectParser::new(ProjectParserOptions { verbose: false })
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
                declared_dependencies_by_language: Some(HashMap::from([("go".to_string(), go)])),
                ..Default::default()
            }),
        })
        .relationships
        .expect("une résolution")
}

#[test]
fn un_import_go_produit_sa_bibliotheque_et_sa_provenance() {
    let r = resoudre();
    let libs: Vec<(String, Option<Provenance>)> = r.external_libraries.values().map(|l| (l.name.clone(), l.provenance)).collect();
    let provenance = |nom: &str| libs.iter().find(|(n, _)| n == nom).map(|(_, p)| *p);
    assert_eq!(provenance("fmt"), Some(Some(Provenance::Builtin)), "{libs:?}");
    assert_eq!(provenance("github.com/Azure/azure-sdk-for-go/sdk/azcore/policy"), Some(Some(Provenance::Dependency)), "un sous-paquet du module requis : {libs:?}");
    assert_eq!(provenance("github.com/azure/azure-sdk-for-go/sdk/azcore"), Some(Some(Provenance::Unknown)), "le même chemin sous une autre casse, par son alias : {libs:?}");
    assert_eq!(provenance("gopkg.in/yaml.v3"), Some(Some(Provenance::Dependency)), "le nom lié de yaml.v3 est yaml : {libs:?}");
    assert_eq!(provenance("github.com/x/mod/v2"), Some(Some(Provenance::Dependency)), "le nom lié de …/mod/v2 est mod : {libs:?}");
}
