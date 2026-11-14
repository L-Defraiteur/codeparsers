//! **Les dépendances déclarées par langue** (14 novembre 2026, plan validé par l'orchestration) : chaque
//! nom se normalise par les variantes de sa langue (`data/imports/<langue>.toml`), et un import ne se
//! compare qu'à la table de sa langue. Un chemin de module Go garde sa casse et ses tirets ; la table de
//! toutes les langues (`declared_dependencies`) reste comparée minuscules, tirets confondus.

use codeparsers::import_resolution::provenance::{provenance_d_un_import_declare, Declares, Provenance};
use codeparsers::import_resolution::regles::Regles;
use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};
use codeparsers::relationship_resolution::types::RelationshipResolverOptions;
use std::collections::{HashMap, HashSet};

fn ensemble(noms: &[&str]) -> HashSet<String> {
    noms.iter().map(|s| s.to_string()).collect()
}

#[test]
fn go_un_module_a_casse_melee_garde_sa_casse() {
    let r = Regles::charger(None);
    let vide = HashSet::new();
    let par_langue = HashMap::from([("go".to_string(), (ensemble(&["github.com/Azure/azure-sdk-for-go"]), HashSet::new()))]);
    let d = Declares { dependances: &vide, paquets_du_projet: &vide, par_langue: Some(&par_langue), prefixes_locaux: None };
    let p = |source: &str| provenance_d_un_import_declare(&r, "main.go", source, false, &d);
    assert_eq!(p("github.com/Azure/azure-sdk-for-go/sdk/azcore"), Provenance::Dependency, "le module requis, sous sa casse");
    assert_eq!(p("github.com/azure/azure-sdk-for-go/sdk/azcore"), Provenance::Unknown, "un autre module, d'une autre casse");
    assert_eq!(p("github.com/Azure/azure_sdk_for_go"), Provenance::Unknown, "un autre module, un souligné au lieu d'un tiret");
}

#[test]
fn go_la_table_de_toutes_les_langues_reste_minuscule() {
    // Sans table par langue, l'ancienne option (normalisée minuscules, tirets confondus) tient encore.
    let r = Regles::charger(None);
    let vide = HashSet::new();
    let tous = ensemble(&["github.com/azure/azure_sdk_for_go"]);
    let d = Declares { dependances: &tous, paquets_du_projet: &vide, par_langue: None, prefixes_locaux: None };
    assert_eq!(provenance_d_un_import_declare(&r, "main.go", "github.com/Azure/azure-sdk-for-go/sdk/azcore", false, &d), Provenance::Dependency);
}

fn bibliotheque_rust(par_langue: HashMap<String, Vec<String>>) -> Option<Provenance> {
    let chemin = "/virtual/lib.rs".to_string();
    let source = "use serde_json::Value;\n\npub fn lire() -> Value {\n    Value::Null\n}\n";
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
                declared_dependencies_by_language: Some(par_langue),
                ..Default::default()
            }),
        })
        .relationships
        .expect("une résolution");
    r.external_libraries.values().find(|l| l.name == "serde_json").and_then(|l| l.provenance)
}

#[test]
fn rust_une_dependance_de_sa_langue_et_pas_d_une_autre() {
    // Le Cargo.lock dit `serde-json` : la variante tiret_souligne de Rust le rapproche de `serde_json`.
    assert_eq!(bibliotheque_rust(HashMap::from([("rust".to_string(), vec!["serde-json".to_string()])])), Some(Provenance::Dependency));
    // Le même nom déclaré pour Python ne fait pas d'un import Rust une dépendance.
    assert_eq!(bibliotheque_rust(HashMap::from([("python".to_string(), vec!["serde-json".to_string()])])), Some(Provenance::Unknown));
}

#[test]
fn csharp_un_espace_de_noms_trouve_son_paquet_nuget_au_plus_long_prefixe() {
    let r = Regles::charger(None);
    let vide = HashSet::new();
    // Les tables par langue portent des noms normalisés par les variantes de la langue (le résolveur le fait).
    let cs = r.de_langue("csharp").expect("les règles de C#");
    let normes = |noms: &[&str]| noms.iter().map(|n| cs.normaliser(n)).collect::<HashSet<String>>();
    let par_langue = HashMap::from([("csharp".to_string(), (normes(&["Newtonsoft.Json", "Serilog", "Serilog.Sinks.Console"]), normes(&["Mon.Outil"])))]);
    let d = Declares { dependances: &vide, paquets_du_projet: &vide, par_langue: Some(&par_langue), prefixes_locaux: None };
    let p = |source: &str| provenance_d_un_import_declare(&r, "a.cs", source, false, &d);
    // `using Newtonsoft.Json.Linq;` vient du paquet Newtonsoft.Json : l'ancienne règle comparait l'espace entier.
    assert_eq!(p("Newtonsoft.Json.Linq"), Provenance::Dependency);
    assert_eq!(p("Serilog.Sinks.Console.Themes"), Provenance::Dependency);
    assert_eq!(p("Newtonsoft.Jsonx"), Provenance::Unknown, "le préfixe se coupe au point");
    assert_eq!(p("Mon.Outil.Interne"), Provenance::Project);
    assert_eq!(p("System.Text.Json"), Provenance::Builtin);
}
