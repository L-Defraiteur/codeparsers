//! **Un `require` relatif se suit jusqu'à son module** (8 novembre 2026). La mesure des locales
//! JavaScript : dans `tools/wasm/src_js/connection.js`, `const PreparedStatement =
//! require("./prepared_statement.js")` liait le nom, et une mention de `PreparedStatement` allait à
//! cette ligne du même fichier au lieu de la classe de `prepared_statement.js` (ticket
//! `2026-11-08-un-require-relatif-s-arrete-a-la-liaison.md`, rag3db).

use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};
use codeparsers::relationship_resolution::types::{RelationshipResolutionResult, RelationshipResolverOptions};
use std::collections::HashMap;

fn resoudre(fichiers: &[(&str, &str)]) -> RelationshipResolutionResult {
    resoudre_avec(fichiers, true)
}

/// `entre_fichiers = false` : les options de rag3weaver (« fichier seul »), qui fait le reste.
fn resoudre_avec(fichiers: &[(&str, &str)], entre_fichiers: bool) -> RelationshipResolutionResult {
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
                resolve_cross_file: Some(entre_fichiers),
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
    // La vraie mention, `new PreparedStatement` dans `prepare`, et pas la ligne du `require`.
    assert!(
        r.relationships.iter().any(|x| x.from_name == "prepare" && x.to_name == "PreparedStatement" && x.to_file == "prepared_statement.js"),
        "la mention de prepare va à la classe"
    );
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

/// `require("os").cpus().length` n'est pas une liaison : la variable reste une variable du fichier.
#[test]
fn un_require_suivi_d_un_acces_n_est_pas_une_liaison() {
    let source = r#""use strict";

const THREADS = require("os").cpus().length;

function lancer() {
  return THREADS;
}

module.exports = lancer;
"#;
    let r = resoudre(&[("build.js", source)]);
    let c = cibles(&r, "THREADS");
    assert!(c.iter().any(|(f, t)| f == "build.js" && t == "variable"), "{c:?}");
}

/// **Une réexportation va à la classe** (c7, 8 novembre 2026) : `module.exports = { PreparedStatement }`
/// dans un index.js suit le `require` jusqu'à la classe, comme une mention dans le code.
#[test]
fn une_reexportation_va_a_la_classe_du_module() {
    let index = r#""use strict";

const PreparedStatement = require("./prepared_statement.js");

module.exports = {
  PreparedStatement,
};
"#;
    let r = resoudre(&[("prepared_statement.js", PREPARED), ("index.js", index)]);
    let c = cibles(&r, "PreparedStatement");
    assert!(c.iter().any(|(f, t)| f == "prepared_statement.js" && t == "class"), "la réexportation va à la classe : {c:?}");
}

/// **Une instance exportée n'est pas sa classe** : `module.exports = new FS()` ; la mention de `FS` dans
/// le module qui la requiert ne va pas à `class FS`.
#[test]
fn une_instance_exportee_n_est_pas_la_classe() {
    let fs = r#""use strict";

class FS {
  lire(p) { return p; }
}

module.exports = new FS();
"#;
    let index = r#""use strict";

const FS = require("./fs.js");

function lire(p) {
  return FS.lire(p);
}

module.exports = lire;
"#;
    let r = resoudre(&[("fs.js", fs), ("index.js", index)]);
    // Depuis index.js : `lire` va à la zone qui écrit l'instance, jamais à la classe (fs.js, lui,
    // consomme sa classe par `new FS()`, et c'est juste).
    let depuis: Vec<_> = r.relationships.iter().filter(|x| x.from_file == "index.js" && x.from_name == "lire" && x.to_file == "fs.js" && format!("{:?}", x.r#type) == "CONSUMES").collect();
    assert!(!depuis.iter().any(|x| x.to_type == "class"), "une instance n'est pas sa classe : {:?}", depuis.iter().map(|x| (&x.to_name, &x.to_type)).collect::<Vec<_>>());
    assert!(depuis.iter().any(|x| x.to_type == "module"), "la mention va à la zone qui écrit l'instance : {:?}", depuis.iter().map(|x| (&x.to_name, &x.to_type)).collect::<Vec<_>>());
}

/// **Un `require` d'un module hors de l'arbre se replie sur sa liaison**, étiquetée « liaison de module
/// externe » : la mention garde une cible, et l'arête dit qu'elle s'arrête à la liaison.
#[test]
fn un_require_externe_se_replie_sur_sa_liaison_etiquetee() {
    let appelant = r#""use strict";

const reader = require("mtga-reader");

function lire() {
  return reader.read();
}

module.exports = lire;
"#;
    let r = resoudre(&[("collect.js", appelant)]);
    // La liaison est la zone de fichier qui contient la ligne du `require` ; l'arête part de `lire`.
    let vers: Vec<_> = r.relationships.iter().filter(|x| format!("{:?}", x.r#type) == "CONSUMES" && x.from_name == "lire" && x.to_file == "collect.js" && x.to_name.starts_with("file_scope")).collect();
    assert!(!vers.is_empty(), "la mention va à la liaison : {:?}", r.relationships.iter().map(|x| (&x.from_name, &x.to_name)).collect::<Vec<_>>());
    assert!(vers.iter().all(|x| x.metadata.as_ref().is_some_and(|m| m.external_binding == Some(true))), "l'arête est étiquetée liaison de module externe");
}

/// **La même règle pour un `import` ES externe** (c7) : la mention se replie sur la zone de sa ligne
/// d'import, étiquetée.
#[test]
fn un_import_externe_se_replie_sur_sa_liaison_etiquetee() {
    let appelant = r#"import reader from "mtga-reader";

export function lire() {
  return reader.read();
}
"#;
    let r = resoudre(&[("collect.js", appelant)]);
    let vers: Vec<_> = r.relationships.iter().filter(|x| format!("{:?}", x.r#type) == "CONSUMES" && x.from_name == "lire" && x.to_file == "collect.js" && x.to_name.starts_with("file_scope")).collect();
    assert!(!vers.is_empty(), "la mention va à la liaison : {:?}", r.relationships.iter().map(|x| (&x.from_name, &x.to_name)).collect::<Vec<_>>());
    assert!(vers.iter().all(|x| x.metadata.as_ref().is_some_and(|m| m.external_binding == Some(true))));
}

/// **En « fichier seul »** (les options de rag3weaver) : la liaison externe se pose quand même, et un
/// import qui aboutit dans le projet est laissé au consommateur (aucune arête vers la zone).
#[test]
fn en_fichier_seul_seule_la_liaison_externe_se_pose() {
    let appelant = r#""use strict";

const reader = require("mtga-reader");
const PreparedStatement = require("./prepared_statement.js");

function lire() {
  return reader.read(new PreparedStatement(1));
}

module.exports = lire;
"#;
    let r = resoudre_avec(&[("collect.js", appelant), ("prepared_statement.js", PREPARED)], false);
    let vers: Vec<_> = r.relationships.iter().filter(|x| format!("{:?}", x.r#type) == "CONSUMES" && x.from_name == "lire" && x.to_name.starts_with("file_scope")).collect();
    assert_eq!(vers.len(), 1, "une seule liaison, celle du module externe : {:?}", r.relationships.iter().map(|x| (&x.from_name, &x.to_name, x.metadata.as_ref().and_then(|m| m.external_binding))).collect::<Vec<_>>());
    assert_eq!(vers[0].metadata.as_ref().and_then(|m| m.external_binding), Some(true));
}
