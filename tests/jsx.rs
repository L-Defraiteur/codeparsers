//! **Un composant employé en balise est un usage** (18 octobre 2026).
//!
//! Un `.tsx` se parsait avec la grammaire TypeScript, sans JSX : `<Button/>` y tombait en nœud
//! `ERROR`, et le code qui en fait une référence ne se déclenchait jamais. La grammaire suit
//! désormais l'extension (`uses_tsx_grammar`). Ticket fermé :
//! `docs/tickets/closed/2026-10-18-codeparsers-parse-le-tsx-avec-la-grammaire-typescript.md` (rag3db).

use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};
use codeparsers::relationship_resolution::types::{RelationshipType, ResolvedRelationship};
use codeparsers::scope_extraction::base_scope_extraction_parser::uses_tsx_grammar;
use std::collections::HashMap;

fn relations(fichiers: &[(&str, &str)]) -> Vec<ResolvedRelationship> {
    let racine = "/virtual";
    let mut contenus = HashMap::new();
    let mut chemins = Vec::new();
    for (nom, source) in fichiers {
        let chemin = format!("{racine}/{nom}");
        contenus.insert(chemin.clone(), source.to_string());
        chemins.push(chemin);
    }
    ProjectParser::new(ProjectParserOptions { verbose: false })
        .parse_project(ParseProjectOptions {
            root: racine.to_string(),
            files: chemins,
            content_map: Some(contenus),
            resolve_relationships: Some(true),
            resolver_options: None,
        })
        .relationships
        .map_or_else(Vec::new, |r| r.relationships)
}

fn consomme(rels: &[ResolvedRelationship], de: &str, vers: &str) -> bool {
    rels.iter().any(|r| r.from_name == de && r.to_name == vers && r.r#type == RelationshipType::CONSUMES)
}

fn liste(rels: &[ResolvedRelationship]) -> Vec<String> {
    rels.iter().map(|r| format!("{:?} {} → {}", r.r#type, r.from_name, r.to_name)).collect()
}

const APP: &str = r#"import { useState } from "react";

function Button(props: { label: string; onClick: () => void }) {
  return <button onClick={props.onClick}>{props.label}</button>;
}

const Panel = () => <div className="panel"><Button label="ok" onClick={() => {}} /></div>;

export function App() {
  const [n, setN] = useState(0);
  return (
    <section>
      <Button label={String(n)} onClick={() => setN(n + 1)} />
      <Panel />
    </section>
  );
}
"#;

#[test]
fn une_balise_de_composant_est_un_usage_du_composant() {
    let rels = relations(&[("app.tsx", APP)]);
    assert!(consomme(&rels, "App", "Button"), "<Button/> dans App : {:#?}", liste(&rels));
    assert!(consomme(&rels, "App", "Panel"), "<Panel/> dans App : {:#?}", liste(&rels));
    assert!(consomme(&rels, "Panel", "Button"), "<Button/> dans Panel : {:#?}", liste(&rels));
}

#[test]
fn une_balise_en_minuscule_n_est_l_usage_de_rien() {
    let rels = relations(&[("app.tsx", APP)]);
    for intrinseque in ["div", "section", "button"] {
        assert!(
            !rels.iter().any(|r| r.to_name == intrinseque),
            "<{intrinseque}> est un élément du DOM : {:#?}",
            liste(&rels)
        );
    }
}

#[test]
fn le_jsx_d_un_fichier_js_est_lu_aussi() {
    let src = "function Item() { return <li/>; }\nexport function List() { return <ul><Item /></ul>; }\n";
    let rels = relations(&[("list.jsx", src)]);
    assert!(consomme(&rels, "List", "Item"), "{:#?}", liste(&rels));
}

#[test]
fn le_nom_membre_d_une_balise_est_l_usage_de_sa_propriete() {
    let src = "const UI = { Card() { return null; } };\nfunction Card() { return null; }\nexport function Page() { return <UI.Card />; }\n";
    let rels = relations(&[("page.tsx", src)]);
    assert!(consomme(&rels, "Page", "Card"), "{:#?}", liste(&rels));
}

#[test]
fn la_grammaire_suit_l_extension() {
    for f in ["a.tsx", "b.jsx", "c.js", "d.mjs", "e.cjs", "F.TSX"] {
        assert!(uses_tsx_grammar(f), "{f}");
    }
    for f in ["a.ts", "b.mts", "c.cts", "d.d.ts"] {
        assert!(!uses_tsx_grammar(f), "{f} : `<T>x` y est une assertion de type");
    }
}

#[test]
fn un_ts_garde_la_grammaire_typescript() {
    // `<Foo>x` dans un `.ts` est une assertion de type, pas une balise : la grammaire TSX la lirait
    // mal. L'appel qui la suit reste lu, et aucune balise n'y apparaît.
    let src = "function helper() { return 1; }\nfunction use(x: unknown) { const y = <number>x; return helper() + y; }\n";
    let rels = relations(&[("cast.ts", src)]);
    assert!(consomme(&rels, "use", "helper"), "{:#?}", liste(&rels));
}
