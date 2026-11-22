//! **A type name is a reference with its place, in the signature and in the body** (22 November 2026, the lot of the
//! signature types).
//!
//! The common visitor recorded `identifier` nodes only: in TypeScript, C, C++ and Go a type is a `type_identifier`, so
//! a type written in a parameter, a return type or a local annotation was never a reference with a position. Only a
//! text fallback over the signature gave the parameter and return types back, without place. In C#, the name of a
//! parameter was read on its first identifier, its type: `Widget w` made « Widget » a parameter name, hiding every
//! `Widget` of the method, and left `w` a reference.

use std::collections::HashMap;

use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};
use codeparsers::scope_extraction::types::ScopeInfo;

fn scopes(chemin: &str, source: &str) -> Vec<ScopeInfo> {
    let chemin = format!("/virtual/{chemin}");
    let a = ProjectParser::new(ProjectParserOptions { verbose: false }).parse_project(ParseProjectOptions {
        root: "/virtual".to_string(),
        files: vec![chemin.clone()],
        content_map: Some(HashMap::from([(chemin, source.to_string())])),
        resolve_relationships: Some(false),
        resolver_options: None,
    });
    a.files.into_values().flat_map(|fa| fa.scopes).collect()
}

/// In each language, `use_it` takes a `Widget`, returns a `Widget` and annotates a local `Widget`.
const CAS: &[(&str, &str, &str)] = &[
    ("mod.ts", "useIt", "interface Widget {\n  n: number;\n}\n\nfunction useIt(w: Widget): Widget {\n  const x: Widget = w;\n  return x;\n}\n"),
    ("mod.c", "use_it", "typedef struct Widget {\n    int n;\n} Widget;\n\nWidget use_it(Widget w) {\n    Widget x = w;\n    return x;\n}\n"),
    ("mod.cpp", "use_it", "struct Widget {\n    int n;\n};\n\nWidget use_it(Widget w) {\n    Widget x = w;\n    return x;\n}\n"),
    ("mod.go", "useIt", "package m\n\ntype Widget struct {\n\tn int\n}\n\nfunc useIt(w Widget) Widget {\n\tvar x Widget = w\n\treturn x\n}\n"),
    ("Mod.cs", "UseIt", "class Widget {\n}\n\nclass M {\n    static Widget UseIt(Widget w) {\n        Widget x = w;\n        return x;\n    }\n}\n"),
];

#[test]
fn every_type_name_of_a_function_is_a_reference_with_its_range() {
    let mut fautes = Vec::new();
    for (chemin, fonction, source) in CAS {
        let s = scopes(chemin, source);
        let Some(f) = s.iter().find(|x| x.name == *fonction) else {
            fautes.push(format!("{chemin}: no scope {fonction}"));
            continue;
        };
        // From the start of the function's declaration: a C return type is written before the name.
        let debut = source[..source.find(fonction).unwrap()].rfind("\n\n").unwrap();
        let attendus: Vec<usize> = source.match_indices("Widget").map(|(i, _)| i).filter(|i| *i > debut).collect();
        let mut lus: Vec<usize> = f.identifier_references.iter().filter(|r| r.identifier == "Widget").filter_map(|r| r.start_byte).collect();
        lus.sort();
        lus.dedup();
        if lus != attendus {
            let sans_place = f.identifier_references.iter().filter(|r| r.identifier == "Widget" && r.start_byte.is_none()).count();
            fautes.push(format!("{chemin}: Widget is written in {fonction} at bytes {attendus:?}, the references place it at {lus:?} ({sans_place} without place)"));
        }
        if f.identifier_references.iter().any(|r| r.identifier == "w") {
            fautes.push(format!("{chemin}: the parameter w is not a reference of {fonction}"));
        }
    }
    assert!(fautes.is_empty(), "{}", fautes.join("\n"));
}
