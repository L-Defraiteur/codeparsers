//! **Une classe exportée est une classe** (les C++ anonymes, P4 de la stèle
//! du produit, 11 octobre 2026). `class RAG3DB_API ExtraTypeInfo { … }` se
//! lisait, pour tree-sitter qui ne connaît pas la macro, comme une
//! *définition de fonction* (type `class RAG3DB_API`, nom `ExtraTypeInfo`,
//! le corps en instructions) : un scope `AnonymousFunction`, la classe et
//! ses membres perdus. C'était la majeure partie des 287 anonymes de rag3db.
//! La macro d'export est masquée avant l'analyse (même longueur : octets et
//! lignes inchangés), le texte des scopes reste l'original.

use std::collections::HashMap;

use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};
use codeparsers::scope_extraction::types::{ScopeInfo, ScopeInfoType};

fn scopes(source: &str) -> Vec<ScopeInfo> {
    let chemin = "/virtual/types.h".to_string();
    let a = ProjectParser::new(ProjectParserOptions { verbose: false }).parse_project(ParseProjectOptions {
        root: "/virtual".to_string(),
        files: vec![chemin.clone()],
        content_map: Some(HashMap::from([(chemin, source.to_string())])),
        resolve_relationships: Some(false),
        resolver_options: None,
    });
    a.files.into_values().flat_map(|fa| fa.scopes).collect()
}

const SRC: &str = "#pragma once\nnamespace rag3db {\nclass RAG3DB_API ExtraTypeInfo {\npublic:\n    virtual ~ExtraTypeInfo() = default;\n    int profondeur() const { return 1; }\n};\nstruct RAG3DB_API GDSConfig : public Base {\n    int x;\n};\nclass Database {\npublic:\n    RAG3DB_API Catalog* getCatalog() { return catalog.get(); }\n};\n}\n";

#[test]
fn une_classe_exportee_est_une_classe() {
    let s = scopes(SRC);
    let noms: Vec<(String, ScopeInfoType)> = s.iter().map(|x| (x.name.clone(), x.r#type.clone())).collect();
    assert!(!noms.iter().any(|(n, _)| n.contains("Anonymous")), "aucun anonyme : {noms:?}");
    assert!(noms.contains(&("ExtraTypeInfo".to_string(), ScopeInfoType::Class)), "{noms:?}");
    assert!(noms.contains(&("GDSConfig".to_string(), ScopeInfoType::Class)), "{noms:?}");
    let methode = s.iter().find(|x| x.name == "profondeur").expect("la méthode de la classe exportée");
    assert_eq!(methode.parent.as_deref(), Some("ExtraTypeInfo"));
}

#[test]
fn une_methode_exportee_a_son_nom() {
    let s = scopes(SRC);
    let m = s.iter().find(|x| x.name == "getCatalog").expect("getCatalog nommée");
    assert_eq!(m.parent.as_deref(), Some("Database"));
}

/// Le masque garde la longueur : la classe commence à sa ligne, et son
/// corps est le texte original.
#[test]
fn le_masque_ne_deplace_rien() {
    let s = scopes(SRC);
    let c = s.iter().find(|x| x.name == "ExtraTypeInfo").expect("la classe");
    assert_eq!(c.scope_start_line, 3);
    assert!(c.content.contains("virtual ~ExtraTypeInfo() = default;"), "{:?}", c.content);
    let m = s.iter().find(|x| x.name == "profondeur").expect("la méthode");
    assert_eq!(m.scope_start_line, 6);
}

/// Les autres formes qui restaient anonymes sur rag3db : l'opérateur de
/// conversion, la spécialisation d'un gabarit (classe et fonction), le
/// `typedef struct { … } Nom;`, et la méthode exportée à type `const`.
#[test]
fn les_autres_formes_ont_un_nom() {
    let src = "#include <cstdint>\nnamespace rag3db {\nstruct int128_t { int64_t haut; };\nint128_t::operator int8_t() const {\n    return 0;\n}\ntemplate<>\nstruct ListTypeResolver<ListOp::Concat> {\n    static int resoudre();\n};\ntemplate<>\ninline int128_t abs<int128_t>(int128_t value) {\n    return value;\n}\nclass ExtensionManager {\npublic:\n    RAG3DB_API const std::vector<int>& getLoadedExtensions() const {\n        return loaded;\n    }\n};\n}\ntypedef struct {\n    void* _connection;\n} rag3db_connection;\ntypedef enum { Rag3dbSuccess = 0, Rag3dbError = 1 } rag3db_state;\n";
    let s = scopes(src);
    let noms: Vec<(String, ScopeInfoType)> = s.iter().map(|x| (x.name.clone(), x.r#type.clone())).collect();
    eprintln!("{noms:?}");
    assert!(!noms.iter().any(|(n, _)| n.starts_with("Anonymous")), "aucun anonyme : {noms:?}");
    for attendu in ["operator int8_t", "ListTypeResolver", "abs", "getLoadedExtensions", "rag3db_connection", "rag3db_state"] {
        assert!(noms.iter().any(|(n, _)| n == attendu), "{attendu} : {noms:?}");
    }
}

/// **Une méthode gabarit est une méthode** : `template<typename T> void
/// serializeValue(…)` dans la classe n'avait pas de scope — invisible tant
/// que la classe exportée était lue comme une fonction (ses méthodes sortaient
/// alors en fonctions), perdue une fois la classe bien lue.
#[test]
fn une_methode_gabarit_est_une_methode() {
    let src = "class RAG3DB_API Serializer {\npublic:\n    template<typename T>\n    void serializeValue(const T& value) {\n        write(value);\n    }\n    void serializeMap(int n) {\n        serializeValue(n);\n    }\n    void write(int x) {}\n};\n";
    let s = scopes(src);
    let m = s.iter().find(|x| x.name == "serializeValue").expect("la méthode gabarit a son scope");
    assert_eq!(m.parent.as_deref(), Some("Serializer"));
}
