//! **Ce qu'un appel de méthode standard dit au résolveur entre fichiers** (ticket rag3db « la marque
//! nom relie un appel de méthode standard à un homonyme », 7 novembre 2026). `reste.is_zero()` après
//! `let reste = WAIT.saturating_sub(…)` : aucun type ne se lit, ni sur place ni ailleurs — la référence
//! le dit (`receiver_type_unread`), pour qu'un consommateur ne la relie pas à l'`is_zero` d'un autre
//! fichier. `Value::Float(f) =>` : une variante, son type se lit par le chemin (`qualifier_type`).

use std::collections::HashMap;

use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};
use codeparsers::scope_extraction::types::IdentifierReference;

fn references(source: &str, scope: &str, nom: &str) -> Vec<IdentifierReference> {
    let chemin = "/virtual/lib.rs".to_string();
    let a = ProjectParser::new(ProjectParserOptions { verbose: false }).parse_project(ParseProjectOptions {
        root: "/virtual".to_string(),
        files: vec![chemin.clone()],
        content_map: Some(HashMap::from([(chemin, source.to_string())])),
        resolve_relationships: Some(false),
        resolver_options: None,
    });
    a.files
        .values()
        .flat_map(|f| f.scopes.iter().filter(|s| s.name == scope).flat_map(|s| s.identifier_references.iter().filter(|r| r.identifier == nom).cloned()))
        .collect()
}

#[test]
fn une_locale_sans_type_lu_le_dit() {
    let src = "use std::time::{Duration, Instant};\nconst WAIT: Duration = Duration::from_secs(1);\npub fn ouvre() {\n    let debut = Instant::now();\n    let reste = WAIT.saturating_sub(debut.elapsed());\n    if !reste.is_zero() {}\n}\n";
    let r = references(src, "ouvre", "is_zero");
    assert_eq!(r.len(), 1, "{r:?}");
    assert!(r[0].qualifier_type.is_none() && r[0].qualifier_deferred.is_none(), "{r:?}");
    assert!(r[0].receiver_type_unread, "reste.is_zero() : aucun type ne se lit — {r:?}");
}

#[test]
fn une_locale_typee_ne_le_dit_pas() {
    // Annotée : le type se lit sur place. (`let t = T;` se lit plus tard, à la résolution ; le
    // consommateur ne prend la marque que sans type lu.)
    let src = "pub struct T;\nimpl T {\n    pub fn f(&self) {}\n}\npub fn g() {\n    let t: T = T;\n    t.f();\n}\n";
    let r = references(src, "g", "f");
    assert_eq!(r.len(), 1, "{r:?}");
    assert_eq!(r[0].qualifier_type.as_deref(), Some("T"), "{r:?}");
    assert!(!r[0].receiver_type_unread, "{r:?}");
}

#[test]
fn une_variante_d_enum_lit_son_type_par_le_chemin() {
    let src = "pub enum Value {\n    Float(f64),\n}\npub fn cellule(v: &Value) -> String {\n    match v {\n        Value::Float(f) => f.to_string(),\n    }\n}\n";
    let r = references(src, "cellule", "Float");
    assert!(!r.is_empty(), "la variante est une référence");
    assert!(r.iter().all(|x| x.qualifier_type.as_deref() == Some("Value")), "Value::Float : le type Value — {r:?}");
}

#[test]
fn un_chemin_de_module_n_est_pas_la_locale_homonyme() {
    let src = "mod metadata {\n    pub fn decode_rows(x: u8) -> u8 { x }\n}\npub fn charge() -> u8 {\n    let metadata = metadata::decode_rows(1);\n    metadata\n}\n";
    let r = references(src, "charge", "decode_rows");
    assert!(!r.is_empty(), "l'appel est une référence");
    assert!(r.iter().all(|x| !x.receiver_type_unread), "metadata:: est un module — {r:?}");
}

#[test]
fn une_methode_du_projet_sur_une_locale_sans_type_ne_le_dit_pas() {
    // `aller` n'est pas une méthode de std : le lien par le nom reste possible entre fichiers.
    let src = "pub fn parcours(ctx: &mut Ctx) -> Option<()> {\n    let n = ctx.prends(\"n\").and_then(|p| conv(p))?;\n    n.aller();\n    n.is_empty();\n    Some(())\n}\n";
    let aller = references(src, "parcours", "aller");
    assert_eq!(aller.len(), 1, "{aller:?}");
    // Une lecture au mieux (`OfType` de `Ctx`, que `and_then` arrête) peut naître : elle ne change rien.
    assert!(aller[0].qualifier_type.is_none() && aller[0].qualifier_deferred.as_ref().is_none_or(|d| d.is_best_effort()), "{aller:?}");
    assert!(!aller[0].receiver_type_unread, "aller : méthode du projet — {aller:?}");
    let vide = references(src, "parcours", "is_empty");
    assert!(vide.iter().all(|x| x.receiver_type_unread), "is_empty : méthode de std — {vide:?}");
}
