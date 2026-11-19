//! **A local variable of C or C++ is not a reference** (rag3db, 19 November 2026). The collection of a scope's locals
//! did not know the C and C++ declarators (`init_declarator`, a plain `declaration`, `for_range_loop`): a variable
//! declared `auto total = …`, `int total = …` or `for (auto& item : …)` stayed among the references, and was linked to
//! a function or a namespace of the same name in its file. The JavaScript stop word `function` hid it for one name
//! (79 false links to `namespace function` in the engine once it went). A call next to it is still a reference.

use std::collections::HashMap;

use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};

fn references(file: &str, scope: &str, source: &str) -> Vec<codeparsers::scope_extraction::types::IdentifierReference> {
    let path = format!("/virtual/{file}");
    let a = ProjectParser::new(ProjectParserOptions { verbose: false }).parse_project(ParseProjectOptions {
        root: "/virtual".to_string(),
        files: vec![path.clone()],
        content_map: Some(HashMap::from([(path, source.to_string())])),
        resolve_relationships: Some(false),
        resolver_options: None,
    });
    a.files.values().flat_map(|f| f.scopes.iter().filter(|s| s.name == scope).flat_map(|s| s.identifier_references.clone())).collect()
}

fn identifiers(file: &str, scope: &str, source: &str) -> Vec<String> {
    references(file, scope, source).into_iter().map(|r| r.identifier).collect()
}

#[test]
fn a_local_variable_of_c_or_cpp_is_not_a_reference() {
    let cases = [
        (
            "C++",
            "lib.cpp",
            "int total() { return 0; }\nint item() { return 1; }\nint compute() { return 2; }\nint start() {\n    auto total = compute();\n    int count = 3;\n    for (auto& item : {1, 2}) { count += item; }\n    return total + count;\n}\n",
        ),
        ("C", "lib.c", "int total(void) { return 0; }\nint compute(void) { return 2; }\nint start(void) {\n    int total = compute();\n    return total;\n}\n"),
    ];
    let mut wrong = Vec::new();
    for (language, file, source) in cases {
        let ids = identifiers(file, "start", source);
        assert!(ids.iter().any(|i| i == "compute"), "{language}: the call is a reference: {ids:?}");
        for local in ["total", "item", "count"] {
            if ids.iter().any(|i| i == local) {
                wrong.push(format!("{language}: the local {local} is a reference ({ids:?})"));
            }
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}

/// **A method called on a C++ local keeps its link by name** (19 November 2026). Recognising the local must not bring
/// in Rust's rule of the untyped receiver (`let t = f(); t.m()` is not linked by name): on the engine it cut 577 links,
/// each to the only declaration of its method (`auto tableSchema = FactorizedTableSchema(); tableSchema.appendColumn(…)`).
#[test]
fn a_method_called_on_a_cpp_local_keeps_its_link_by_name() {
    // The shape of the engine (aggregate_hash_table.cpp): in a method defined out of its class, a local bound to a
    // construction whose type is deferred.
    let source = "FactorizedTableSchema AggregateHashTableUtils::getTableSchemaForKeys() {\n    auto tableSchema = FactorizedTableSchema();\n    auto columnSchema = ColumnSchema(false, 0, 8);\n    tableSchema.appendColumn(std::move(columnSchema));\n    return tableSchema;\n}\n";
    let refs = references("lib.cpp", "getTableSchemaForKeys", source);
    let append = refs.iter().find(|r| r.identifier == "appendColumn").unwrap_or_else(|| panic!("appendColumn is a reference: {refs:?}"));
    assert!(append.qualifier_deferred.is_some(), "the shape of the engine: the receiver's type is deferred ({append:?})");
    assert!(
        !append.untyped_receiver && !append.receiver_type_unread,
        "the call on the local keeps its link by name (untyped_receiver {}, receiver_type_unread {})",
        append.untyped_receiver,
        append.receiver_type_unread
    );
}
