//! **Un `require` relatif se suit jusqu'à son module** (8 novembre 2026). La mesure des locales
//! JavaScript : dans `tools/wasm/src_js/connection.js`, `const PreparedStatement =
//! require("./prepared_statement.js")` liait le nom, et une mention de `PreparedStatement` allait à
//! cette ligne du même fichier au lieu de la classe de `prepared_statement.js` (ticket
//! `2026-11-08-un-require-relatif-s-arrete-a-la-liaison.md`, rag3db).

use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};
use codeparsers::relationship_resolution::types::{RelationshipResolutionResult, RelationshipResolverOptions};
use std::collections::HashMap;

fn resoudre(fichiers: &[(&str, &str)]) -> RelationshipResolutionResult {
    let mut contenus = HashMap::new();
    let mut chemins = Vec::new();
    for (nom, source) in fichiers {
        let chemin = format!("/virtual/{nom}");
        contenus.insert(chemin.clone(), source.to_string());
        chemins.push(chemin);
    }
    ProjectParser::new(ProjectParserOptions { verbose: false })
        .parse_project(ParseProjectOptions {
            root: "/virtual".to_string(),
            files: chemins,
            content_map: Some(contenus),
            resolve_relationships: Some(true),
            resolver_options: Some(RelationshipResolverOptions {
                include_file_level_refs: Some(false),
                include_child_refs: Some(false),
                ..Default::default()
            }),
        })
        .relationships
        .expect("une résolution")
}

/// Les cibles d'un nom : (fichier de la cible, type de la cible), pour toute relation vers ce nom.
fn cibles(r: &RelationshipResolutionResult, nom: &str) -> Vec<(String, String)> {
    let mut v: Vec<(String, String)> = r.relationships.iter().filter(|x| x.to_name == nom).map(|x| (x.to_file.clone(), x.to_type.clone())).collect();
    v.sort();
    v.dedup();
    v
}

const PREPARED: &str = r#""use strict";

class PreparedStatement {
  constructor(id) {
    this._id = id;
  }
}

module.exports = PreparedStatement;
"#;

const CONNECTION_REQUIRE: &str = r#""use strict";

const PreparedStatement = require("./prepared_statement.js");

class Connection {
  async prepare(statement) {
    const preparedStatement = new PreparedStatement(statement);
    return preparedStatement;
  }
}

module.exports = Connection;
"#;

const CONNECTION_IMPORT: &str = r#"import PreparedStatement from "./prepared_statement.js";

class Connection {
  async prepare(statement) {
    const preparedStatement = new PreparedStatement(statement);
    return preparedStatement;
  }
}

export default Connection;
"#;

/// Le témoin de la forme ES : un `import` se suit déjà jusqu'à la classe.
#[test]
fn un_import_va_a_la_classe_du_module() {
    let r = resoudre(&[("prepared_statement.js", PREPARED), ("connection.js", CONNECTION_IMPORT)]);
    let c = cibles(&r, "PreparedStatement");
    assert!(c.iter().any(|(f, t)| f == "prepared_statement.js" && t == "class"), "{c:?}");
}

#[test]
fn un_require_relatif_va_a_la_classe_du_module() {
    let r = resoudre(&[("prepared_statement.js", PREPARED), ("connection.js", CONNECTION_REQUIRE)]);
    let c = cibles(&r, "PreparedStatement");
    assert!(c.iter().any(|(f, t)| f == "prepared_statement.js" && t == "class"), "la mention va à la classe : {c:?}");
    assert!(!c.iter().any(|(f, _)| f == "connection.js"), "jamais à la liaison du même fichier : {c:?}");
}

#[test]
fn un_require_destructure_va_aux_exports_nommes() {
    let outils = r#""use strict";

function lire(x) { return x; }
function ecrire(x) { return x; }

module.exports = { lire, ecrire };
"#;
    let appelant = r#""use strict";

const { lire, ecrire } = require("./outils.js");

function copier(x) {
  return ecrire(lire(x));
}

module.exports = copier;
"#;
    let r = resoudre(&[("outils.js", outils), ("copier.js", appelant)]);
    for nom in ["lire", "ecrire"] {
        let c = cibles(&r, nom);
        assert!(c.iter().any(|(f, _)| f == "outils.js"), "{nom} va à outils.js : {c:?}");
        assert!(!c.iter().any(|(f, _)| f == "copier.js"), "{nom} ne va pas à la liaison : {c:?}");
    }
}

/// Un module hors de l'arbre (`require("path")`) : rien à suivre, aucune arête vers un fichier de l'arbre.
#[test]
fn un_require_d_un_module_externe_ne_va_a_aucun_fichier_de_l_arbre() {
    let appelant = r#""use strict";

const path = require("path");

function joindre(a, b) {
  return path.join(a, b);
}

module.exports = joindre;
"#;
    let r = resoudre(&[("joindre.js", appelant), ("prepared_statement.js", PREPARED)]);
    let c = cibles(&r, "path");
    assert!(!c.iter().any(|(f, _)| f == "prepared_statement.js"), "{c:?}");
}
