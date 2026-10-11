//! **Un lien posé par le nom seul, dans le fichier, sur un receveur non
//! typé, se dit par le nom** (décision de l'orchestration pour Lucie, 11
//! octobre 2026). `let prepared = self.prepared(); prepared.describe()` —
//! un receveur dont aucun type ne se lit — était relié à la seule méthode
//! `describe` du fichier, comme un lien sûr (marqué « fichier »). Le lien reste (la moitié de
//! ces liens est juste) ; la relation le dit : `by_name`. Un lien par le
//! type, ou par `self`, ne l'est pas.

use std::collections::HashMap;

use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};
use codeparsers::relationship_resolution::types::RelationshipResolverOptions;

/// (de, vers, par le nom) des relations d'appel du fichier, en fichier seul.
fn relations(source: &str) -> Vec<(String, String, bool)> {
    let chemin = "/virtual/lib.rs".to_string();
    let a = ProjectParser::new(ProjectParserOptions { verbose: false }).parse_project(ParseProjectOptions {
        root: "/virtual".to_string(),
        files: vec![chemin.clone()],
        content_map: Some(HashMap::from([(chemin, source.to_string())])),
        resolve_relationships: Some(true),
        resolver_options: Some(RelationshipResolverOptions { resolve_cross_file: Some(false), ..Default::default() }),
    });
    let mut v: Vec<_> = a
        .relationships
        .expect("les relations")
        .relationships
        .iter()
        .filter(|x| format!("{:?}", x.r#type) == "CONSUMES")
        .map(|x| (x.from_name.clone(), x.to_name.clone(), x.metadata.as_ref().and_then(|m| m.by_name).unwrap_or(false)))
        .collect();
    v.sort();
    v
}

const SRC: &str = "pub enum Resolution { Fichier }\n\
impl Resolution {\n    pub fn as_str(&self) -> &str { \"fichier\" }\n    pub fn libelle(&self) -> &str { self.as_str() }\n}\n\
pub struct Prepared;\n\
impl Prepared {\n    pub fn describe(&self) -> u8 { 0 }\n}\n\
pub struct Backend;\n\
impl Backend {\n    fn prepared(&self) -> Prepared { Prepared }\n    pub fn cles(&self) -> u8 {\n        let prepared = self.prepared();\n        prepared.describe()\n    }\n}\n\
pub fn sure(r: Resolution) -> String {\n    r.as_str().to_string()\n}\n";

#[test]
fn un_lien_par_le_nom_sur_un_receveur_non_type_le_dit() {
    let r = relations(SRC);
    assert!(r.iter().any(|x| x.0 == "cles" && x.1 == "describe" && x.2), "prepared.describe() : relié, par le nom — {r:?}");
}

#[test]
fn un_lien_par_le_type_ou_par_self_ne_le_dit_pas() {
    let r = relations(SRC);
    assert!(r.iter().any(|x| x.0 == "sure" && x.1 == "as_str" && !x.2), "r: Resolution, par le type — {r:?}");
    assert!(r.iter().any(|x| x.0 == "libelle" && x.1 == "as_str" && !x.2), "self.as_str(), par self — {r:?}");
}
