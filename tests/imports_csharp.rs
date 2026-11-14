//! **Les `using` C# produisent leurs bibliothèques** (14 novembre 2026, ticket « un using C# ne produit
//! aucune bibliothèque », voies 1 et 3 de l'orchestration) : le parseur C# lisait ses imports avec les
//! expressions de JavaScript. Chaque `using` se lit sur l'arbre et donne une bibliothèque, avec sa
//! provenance, sans nom lié (il verse les types de l'espace dans la portée : rien n'est deviné) ; un
//! alias (`using J = Serilog.Log;`) lie son nom. `using static` verse des membres : pas de nom lié.

use codeparsers::import_resolution::provenance::Provenance;
use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};
use codeparsers::relationship_resolution::types::RelationshipResolverOptions;
use std::collections::HashMap;

const SOURCE: &str = "using System.Text.Json;\nusing Newtonsoft.Json.Linq;\nusing Mon.Outil.Interne;\nusing J = Serilog.Log;\nusing static System.Math;\n\nnamespace Mon.Outil\n{\n    public class Lecteur\n    {\n        public JObject Lire(string s) { J.Information(s); return JObject.Parse(s); }\n    }\n}\n";

#[test]
fn un_using_csharp_produit_sa_bibliotheque_et_sa_provenance() {
    let chemin = "/virtual/Lecteur.cs".to_string();
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
                declared_dependencies_by_language: Some(HashMap::from([("csharp".to_string(), vec!["Newtonsoft.Json".to_string(), "Serilog".to_string()])])),
                project_packages_by_language: Some(HashMap::from([("csharp".to_string(), vec!["Mon.Outil".to_string()])])),
                ..Default::default()
            }),
        })
        .relationships
        .expect("une résolution");
    let libs: Vec<(String, Option<Provenance>)> = r.external_libraries.values().map(|l| (l.name.clone(), l.provenance)).collect();
    let provenance = |nom: &str| libs.iter().find(|(n, _)| n == nom).map(|(_, p)| *p);
    assert_eq!(provenance("System.Text.Json"), Some(Some(Provenance::Builtin)), "{libs:?}");
    assert_eq!(provenance("Newtonsoft.Json.Linq"), Some(Some(Provenance::Dependency)), "le paquet du .csproj au plus long préfixe : {libs:?}");
    assert_eq!(provenance("Mon.Outil.Interne"), Some(Some(Provenance::Project)), "un espace du projet : {libs:?}");
    assert_eq!(provenance("Serilog.Log"), Some(Some(Provenance::Dependency)), "l'alias J : {libs:?}");
    assert_eq!(provenance("System.Math"), Some(Some(Provenance::Builtin)), "using static : {libs:?}");
}
