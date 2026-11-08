//! **Une locale ne se résout que depuis sa portée** (8 novembre 2026). Le rejugement de l'oracle
//! TypeScript (21 arêtes sur 300) : dans `lucivy-worker.js`, les `case` d'un `switch` déclarent
//! chacun `const ctx` ; une mention de `ctx` dans un `case` était reliée à la `const ctx` d'un autre
//! `case`, le premier homonyme du fichier (ticket
//! `2026-11-08-la-marque-fichier-relie-une-locale-javascript-au-premier-homonyme.md`, rag3db).

use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};
use codeparsers::relationship_resolution::types::{RelationshipResolutionResult, RelationshipResolverOptions};
use std::collections::HashMap;

fn resoudre(nom: &str, source: &str) -> RelationshipResolutionResult {
    let chemin = format!("/virtual/{nom}");
    ProjectParser::new(ProjectParserOptions { verbose: false })
        .parse_project(ParseProjectOptions {
            root: "/virtual".to_string(),
            files: vec![chemin.clone()],
            content_map: Some(HashMap::from([(chemin, source.to_string())])),
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

/// La forme de `lucivy-worker.js` : un gestionnaire en fonction fléchée assignée, un `switch` dont
/// chaque `case` est un bloc qui déclare sa `const ctx`.
const WORKER: &str = r#"function getCtx(path) {
    return indexes.get(path);
}

self.onmessage = async (e) => {
    const { type, ...args } = e.data;
    switch (type) {
        case 'create': {
            const ctx = await Module.ccall('lucivy_create', 'number',
                ['string'],
                [args.path], { async: true });
            indexes.set(args.path, ctx);
            break;
        }
        case 'open': {
            const ctx = await Module.ccall('lucivy_open', 'number', ['number'], [1], { async: true });
            indexes.set(args.path, ctx);
            break;
        }
        case 'drain': {
            const ctx = getCtx(args.path);
            const res = await callStr('lucivy_drain', ctx);
            break;
        }
    }
};
"#;

/// Les lignes (1-based) des trois `const ctx` et des mentions qui les suivent.
const CTX_CREATE: usize = 9;
const MENTION_OPEN: usize = 17;
const MENTION_DRAIN: usize = 22;

#[test]
fn une_mention_d_une_locale_ne_va_pas_a_l_homonyme_d_un_autre_bloc() {
    let r = resoudre("worker.js", WORKER);
    let mut fautes = Vec::new();
    for rel in &r.relationships {
        if rel.to_name != "ctx" {
            continue;
        }
        let Some(cible) = r.uuid_mapping.get(&rel.to_uuid) else { continue };
        let lignes = rel.metadata.as_ref().map(|m| m.sites.iter().filter_map(|s| s.line).collect::<Vec<_>>()).unwrap_or_default();
        for l in lignes {
            // Une mention n'atteint une `const ctx` que si elle est dans le bloc de cette `const`.
            if cible.start_line == CTX_CREATE && (l == MENTION_OPEN || l == MENTION_DRAIN) {
                fautes.push(format!("{} (ligne {l}) → ctx des lignes {}-{}", rel.from_name, cible.start_line, cible.end_line));
            }
        }
    }
    assert!(fautes.is_empty(), "une mention reliée à la `const ctx` d'un autre `case` : {fautes:#?}");
}

/// Les cibles de `ctx`, par ligne de mention : (ligne de la mention, ligne de la `const ctx` visée).
fn cibles_de(r: &RelationshipResolutionResult, nom: &str) -> Vec<(usize, usize)> {
    let mut v: Vec<(usize, usize)> = r
        .relationships
        .iter()
        .filter(|rel| rel.to_name == nom)
        .filter_map(|rel| Some((rel, r.uuid_mapping.get(&rel.to_uuid)?)))
        .flat_map(|(rel, cible)| {
            rel.metadata.iter().flat_map(|m| m.sites.iter().filter_map(|s| s.line)).map(move |l| (l, cible.start_line)).collect::<Vec<_>>()
        })
        .collect();
    v.sort();
    v.dedup();
    v
}

#[test]
fn chaque_mention_va_a_la_locale_de_son_bloc() {
    let r = resoudre("worker.js", WORKER);
    let v = cibles_de(&r, "ctx");
    for attendu in [(12, CTX_CREATE), (MENTION_OPEN, 16), (MENTION_DRAIN, 21)] {
        assert!(v.contains(&attendu), "{attendu:?} absent de {v:?}");
    }
}

#[test]
fn hors_du_bloc_aucune_locale_mais_une_globale_se_voit_partout() {
    let source = r#"const GLOBALE = make(0);

self.onmessage = async (e) => {
    if (e.ok) {
        const ctx = make(1);
        use(ctx, GLOBALE);
    }
    finish(ctx, GLOBALE);
};

function plus_loin() {
    return GLOBALE;
}
"#;
    let r = resoudre("w.js", source);
    let ctx = cibles_de(&r, "ctx");
    assert!(ctx.contains(&(6, 5)), "la mention du bloc va à sa locale : {ctx:?}");
    assert!(!ctx.iter().any(|(l, _)| *l == 8), "après la fin du bloc, `ctx` n'est plus visible : {ctx:?}");
    let globale = cibles_de(&r, "GLOBALE");
    assert!(globale.iter().any(|(l, c)| *l == 8 && *c == 1), "une globale se voit hors des blocs : {globale:?}");
}

#[test]
fn une_var_se_voit_jusqu_a_la_fin_de_sa_fonction() {
    let source = r#"self.onmessage = async (e) => {
    if (e.ok) {
        var tampon = make(1);
    }
    finish(tampon);
};
"#;
    let r = resoudre("v.js", source);
    let v = cibles_de(&r, "tampon");
    assert!(v.contains(&(5, 3)), "une `var` passe la fin de son bloc : {v:?}");
}
