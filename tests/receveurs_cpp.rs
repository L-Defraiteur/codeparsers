//! **Le type d'un receveur C++, lu dans son fichier** — le même mécanisme
//! qu'en Rust (`receveur`) : la liaison visible à la position de l'appel,
//! son type écrit en entier, pelé par une chaîne dont le retour est certain.
//!
//! Banc du 10 octobre : la méthode sur un receveur non typé est le premier
//! trou de codeparsers sur le C++ de rag3db (20 713 non résolues), et
//! `NodeTable::update` n'avait aucun appelant relié.

use std::collections::HashMap;

use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};
use codeparsers::scope_extraction::types::IdentifierReference;

fn refs(source: &str) -> Vec<(String, IdentifierReference)> {
    let chemin = "/virtual/table.cpp".to_string();
    let a = ProjectParser::new(ProjectParserOptions { verbose: false }).parse_project(ParseProjectOptions {
        root: "/virtual".to_string(),
        files: vec![chemin.clone()],
        content_map: Some(HashMap::from([(chemin, source.to_string())])),
        resolve_relationships: Some(false),
        resolver_options: None,
    });
    a.files.values().flat_map(|fa| fa.scopes.iter().flat_map(|s| s.identifier_references.iter().map(|r| (s.name.clone(), r.clone())))).collect()
}

const SOURCE: &str = r#"#include <memory>

namespace rag3db {
namespace storage {

class NodeTable {
public:
    void update(int t);
};

class Table {
public:
    template<class T>
    T& cast();
};

class Manager {
public:
    Table* getTable(int id);
    static Manager* Get(int c);
};

void f(NodeTable& a, const NodeTable* b, std::unique_ptr<NodeTable> c, int id) {
    a.update(1);
    b->update(2);
    c->update(3);
    auto d = std::make_unique<NodeTable>();
    d->update(4);
    auto* e = new NodeTable();
    e->update(5);
    auto& g = Manager::Get(0)->getTable(id)->cast<NodeTable>();
    g.update(6);
    auto h = static_cast<NodeTable*>(nullptr);
    h->update(7);
    NodeTable i;
    i.update(8);
}

} // namespace storage
} // namespace rag3db
"#;

#[test]
fn chaque_forme_de_receveur_se_type() {
    let r = refs(SOURCE);
    let appels: Vec<(usize, Option<String>, Option<String>)> =
        r.iter().filter(|(s, x)| s == "f" && x.identifier == "update").map(|(_, x)| (x.line, x.qualifier.clone(), x.qualifier_type.clone())).collect();
    eprintln!("{appels:#?}");
    assert_eq!(appels.len(), 8, "les huit appels sont relevés : {appels:#?}");
    let non_types: Vec<&(usize, Option<String>, Option<String>)> = appels.iter().filter(|(_, _, t)| t.as_deref() != Some("NodeTable")).collect();
    assert!(non_types.is_empty(), "receveurs non typés NodeTable : {non_types:#?}");
}

#[test]
fn les_champs_d_une_classe_disent_leur_type() {
    use codeparsers::scope_extraction::types::ClassMemberInfoMemberType as M;
    let src = "#include <memory>\n\nclass Holder {\npublic:\n    void go();\nprivate:\n    LocalTable* localTable;\n    NodeTable& ref;\n    std::unique_ptr<RelTable> owned;\n    int a, b;\n    char nom[16];\n    Index idx = Index();\n};\n";
    let chemin = "/virtual/holder.h".to_string();
    let a = ProjectParser::new(ProjectParserOptions { verbose: false }).parse_project(ParseProjectOptions {
        root: "/virtual".to_string(),
        files: vec![chemin.clone()],
        content_map: Some(HashMap::from([(chemin, src.to_string())])),
        resolve_relationships: Some(false),
        resolver_options: None,
    });
    let classe = a.files.values().flat_map(|fa| fa.scopes.iter()).find(|s| s.name == "Holder").expect("Holder");
    let champs: Vec<(String, Option<String>)> =
        classe.members.iter().flatten().filter(|m| m.member_type == M::Property).map(|m| (m.name.clone(), m.r#type.clone())).collect();
    let attendu: Vec<(String, Option<String>)> = [
        ("localTable", "LocalTable"),
        ("ref", "NodeTable"),
        ("owned", "std::unique_ptr<RelTable>"),
        ("a", "int"),
        ("b", "int"),
        ("nom", "char"),
        ("idx", "Index"),
    ]
    .iter()
    .map(|(n, t)| (n.to_string(), Some(t.to_string())))
    .collect();
    assert_eq!(champs, attendu);
}

#[test]
fn une_chaine_de_champs_se_suit_pas_a_pas() {
    // set_executor.cpp : `tableInfo.table->update()`, `tableInfo` membre
    // implicite, `table` champ de ce membre.
    let src = "class NodeTable {\npublic:\n    void update(int v);\n};\n\nstruct Info {\n    NodeTable* table;\n};\n\nclass Exec {\npublic:\n    void set();\nprivate:\n    Info tableInfo;\n};\n\nvoid Exec::set() {\n    tableInfo.table->update(1);\n    this->tableInfo.table->update(2);\n}\n";
    let r = refs(src);
    let types: Vec<Option<String>> = r.iter().filter(|(s, x)| s == "set" && x.identifier == "update").map(|(_, x)| x.qualifier_type.clone()).collect();
    assert_eq!(types, vec![Some("NodeTable".to_string()), Some("NodeTable".to_string())], "{r:#?}");
}
