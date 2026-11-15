//! **Chaque homonyme d'un fichier a son `DEFINED_IN`** (ticket du 18 octobre 2026, revu par le banc le
//! 15 novembre) : dans `json_utils.h`, des quatre constructeurs de `JsonMutWrapper`, seul le dernier
//! avait son arête vers le fichier. La recette du banc : la classe telle qu'elle est dans le dépôt.

use std::collections::HashMap;

use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};
use codeparsers::relationship_resolution::types::{RelationshipResolverOptions, RelationshipType};

const SRC: &str = "#pragma once\n\nclass JsonMutWrapper {\npublic:\n    JsonMutWrapper() : ptr{yyjson_mut_doc_new(nullptr)} {}\n    explicit JsonMutWrapper(yyjson_mut_doc* ptr) : ptr{ptr} {}\n    ~JsonMutWrapper();\n    JsonMutWrapper(JsonMutWrapper& other) = delete;\n    JsonMutWrapper(JsonMutWrapper&& other) {\n        ptr = other.ptr;\n        other.ptr = nullptr;\n    }\n\n    yyjson_mut_doc* ptr;\n};\n";

#[test]
fn chaque_constructeur_homonyme_a_son_defined_in() {
    let chemin = "/virtual/json_utils.h".to_string();
    let a = ProjectParser::new(ProjectParserOptions { verbose: false }).parse_project(ParseProjectOptions {
        root: "/virtual".to_string(),
        files: vec![chemin.clone()],
        content_map: Some(HashMap::from([(chemin.clone(), SRC.to_string())])),
        resolve_relationships: Some(true),
        resolver_options: Some(RelationshipResolverOptions {
            include_file_level_refs: Some(false),
            include_child_refs: Some(false),
            resolve_cross_file: Some(false),
            project_files: Some(vec![chemin.clone()]),
            ..Default::default()
        }),
    });
    let scopes: Vec<_> = a.files.values().flat_map(|fa| fa.scopes.iter()).filter(|s| s.name == "JsonMutWrapper").collect();
    let r = a.relationships.as_ref().expect("une résolution");
    let uuids: Vec<String> = r.uuid_mapping.iter().filter(|(_, e)| e.name == "JsonMutWrapper").map(|(u, _)| u.clone()).collect();
    let defined_in: Vec<&str> = r.relationships.iter().filter(|x| x.r#type == RelationshipType::DEFINEDIN && x.from_name == "JsonMutWrapper").map(|x| x.from_uuid.as_str()).collect();
    for s in &scopes {
        eprintln!("SCOPE {:?} l.{} : {}", s.r#type, s.scope_start_line, s.signature);
    }
    eprintln!("UUIDS {} distincts sur {} scopes ; DEFINED_IN {}", uuids.len(), scopes.len(), defined_in.len());
    assert_eq!(uuids.len(), scopes.len(), "un uuid par scope homonyme");
    assert_eq!(defined_in.len(), scopes.len(), "un DEFINED_IN par scope homonyme");
}
