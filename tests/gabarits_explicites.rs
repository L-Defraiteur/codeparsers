//! **Le type d'un receveur tiré d'un gabarit à arguments explicites**, et le
//! `this` implicite. `ku_dynamic_cast<ProcessorTask*>(task)->sink->f()` : le
//! retour `TO` de `ku_dynamic_cast` devient `ProcessorTask*`, son champ
//! `sink` un `PhysicalOperator*`, et l'appel se relie à
//! `PhysicalOperator::f` (processor.cpp:71, retours du cœur, 11 octobre
//! 2026). Dans rag3db/src, environ 900 receveurs passent par un tel gabarit
//! (`ku_dynamic_cast`, `constCast`, `ptrCast`, `cast`).

use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};
use codeparsers::relationship_resolution::types::{RelationshipResolverOptions, RelationshipType};
use codeparsers::scope_extraction::usage::{template_call, TemplateCall};
use std::collections::HashMap;

/// Les CONSUMES (« fichier:nom » → « fichier:nom », vers soi-même ?) des
/// fichiers donnés.
fn consumes(fichiers: &[(&str, &str)], entre_fichiers: bool) -> Vec<(String, String, bool)> {
    let mut contenus = HashMap::new();
    let mut chemins = Vec::new();
    for (nom, src) in fichiers {
        let c = format!("/virtual/{nom}");
        contenus.insert(c.clone(), src.to_string());
        chemins.push(c);
    }
    let analyse = ProjectParser::new(ProjectParserOptions { verbose: false }).parse_project(ParseProjectOptions {
        root: "/virtual".into(),
        files: chemins,
        content_map: Some(contenus),
        resolve_relationships: Some(true),
        resolver_options: Some(RelationshipResolverOptions { resolve_cross_file: Some(entre_fichiers), ..Default::default() }),
    });
    analyse
        .relationships
        .map(|r| r.relationships)
        .unwrap_or_default()
        .into_iter()
        .filter(|r| r.r#type == RelationshipType::CONSUMES)
        .map(|r| (format!("{}:{}", r.from_file, r.from_name), format!("{}:{}", r.to_file, r.to_name), r.from_uuid == r.to_uuid))
        .collect()
}

const OP_H: &str = "#pragma once\nstruct Ctx {};\nclass PhysicalOperator {\npublic:\n    virtual ~PhysicalOperator() = default;\n    virtual void acquireLocks(Ctx* ctx) {}\n};\nclass NodeBatchInsert : public PhysicalOperator {\npublic:\n    void acquireLocks(Ctx* ctx) override;\n};\ntemplate<typename TO, typename FROM>\nTO ku_dynamic_cast(FROM old) { return dynamic_cast<TO>(old); }\nstruct Task { Task* children[2]; };\nstruct ProcessorTask : Task { PhysicalOperator* sink; };\nclass QueryProcessor {\npublic:\n    void acquireLocks(Task* task, Ctx* ctx);\n};\n";
const OP_CPP: &str = "#include \"op.h\"\nvoid NodeBatchInsert::acquireLocks(Ctx* ctx) {}\nvoid QueryProcessor::acquireLocks(Task* task, Ctx* ctx) {\n    for (auto* child : task->children) {\n        acquireLocks(child, ctx);\n    }\n    ku_dynamic_cast<ProcessorTask*>(task)->sink->acquireLocks(ctx);\n}\n";

#[test]
fn le_champ_d_un_ku_dynamic_cast_type_l_appel_et_la_recursion_ne_retombe_pas_sur_un_homonyme() {
    let rels = consumes(&[("op.h", OP_H), ("op.cpp", OP_CPP)], true);
    let depuis: Vec<&(String, String, bool)> = rels.iter().filter(|(de, _, _)| de == "op.cpp:acquireLocks").collect();
    assert!(depuis.iter().any(|(_, vers, _)| vers == "op.h:acquireLocks"), "vers PhysicalOperator::acquireLocks (op.h) : {rels:?}");
    assert!(depuis.iter().any(|(_, vers, soi)| vers == "op.cpp:acquireLocks" && *soi), "la récursion, vers soi : {rels:?}");
    assert!(!depuis.iter().any(|(_, vers, soi)| vers == "op.cpp:acquireLocks" && !*soi), "jamais NodeBatchInsert::acquireLocks : {rels:?}");
}

#[test]
fn une_methode_gabarit_constcast_type_son_resultat() {
    let src = "struct State {\n    template<typename T>\n    const T& constCast() const { return *static_cast<const T*>(this); }\n};\nstruct NodeState : State { int columnID; void touch() const {} };\nvoid update(State& s) {\n    s.constCast<NodeState>().touch();\n}\n";
    let rels = consumes(&[("s.cpp", src)], false);
    assert!(rels.iter().any(|(de, vers, _)| de == "s.cpp:update" && vers == "s.cpp:touch"), "update → NodeState::touch : {rels:?}");
}

#[test]
fn un_cast_du_langage_type_son_resultat() {
    let src = "struct Op { void run() {} };\nstruct Autre { void run() {} };\nvoid f(void* p) {\n    static_cast<Op*>(p)->run();\n}\n";
    let rels = consumes(&[("c.cpp", src)], false);
    let vers: Vec<&String> = rels.iter().filter(|(de, _, _)| de == "c.cpp:f").map(|(_, v, _)| v).collect();
    assert_eq!(vers.len(), 1, "un seul run, celui de Op : {rels:?}");
}

#[test]
fn un_appel_sans_receveur_vise_une_methode_soeur_avant_un_homonyme() {
    let src = "struct A { void aide(); void run(); };\nstruct B { void aide(); };\nvoid B::aide() {}\nvoid A::aide() {}\nvoid A::run() {\n    aide();\n}\n";
    let rels = consumes(&[("a.cpp", src)], false);
    let vers: Vec<&String> = rels.iter().filter(|(de, _, _)| de == "a.cpp:run").map(|(_, v, _)| v).collect();
    assert_eq!(vers.len(), 1, "{rels:?}");
}

#[test]
fn le_lecteur_de_l_appel_de_gabarit() {
    assert_eq!(
        template_call("ku_dynamic_cast<ProcessorTask*>(task)->sink"),
        Some(TemplateCall { receiver: None, function: "ku_dynamic_cast".into(), args: vec!["ProcessorTask*".into()], fields: vec!["sink".into()] })
    );
    assert_eq!(
        template_call("common::ku_dynamic_cast<const T&, int>(*this)"),
        Some(TemplateCall { receiver: None, function: "ku_dynamic_cast".into(), args: vec!["const T&".into(), "int".into()], fields: vec![] })
    );
    assert_eq!(
        template_call("state.constCast<NodeTableUpdateState>().nodeIDVector"),
        Some(TemplateCall { receiver: Some("state".into()), function: "constCast".into(), args: vec!["NodeTableUpdateState".into()], fields: vec!["nodeIDVector".into()] })
    );
    assert_eq!(template_call("a->ptrCast<X>()->get()"), None, "un appel après les champs n'est pas lu");
    assert_eq!(template_call("a < b"), None);
    assert_eq!(template_call("foo(x)"), None);
}

#[test]
fn une_surcharge_de_son_propre_nom_est_un_appel_et_la_recursion_seule_n_en_fait_pas() {
    // `f(a)` dans `f(int, int)` appelle l'autre surcharge : une vraie arête.
    let src = "struct S {\n    void f(int a);\n    void f(int a, int b);\n};\nvoid S::f(int a) {}\nvoid S::f(int a, int b) {\n    f(a);\n}\nstruct T { void g(); };\nstruct U { void g(); };\nvoid U::g() {}\nvoid T::g() {\n    g();\n}\n";
    let rels = consumes(&[("s.cpp", src)], false);
    assert!(rels.iter().any(|(de, vers, soi)| de == "s.cpp:f" && vers == "s.cpp:f" && !*soi), "la surcharge : {rels:?}");
    // `g()` dans `T::g` : la récursion (vers soi), pas `U::g`.
    assert!(rels.iter().any(|(de, vers, soi)| de == "s.cpp:g" && vers == "s.cpp:g" && *soi), "la récursion : {rels:?}");
    assert!(!rels.iter().any(|(de, vers, soi)| de == "s.cpp:g" && vers == "s.cpp:g" && !*soi), "pas d'homonyme d'une autre classe : {rels:?}");
}
