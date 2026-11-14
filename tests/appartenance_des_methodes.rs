//! **L'appartenance d'une méthode hors du bloc de son type** (14 novembre 2026, la navigation members
//! d'archi) : elle lit `parent` (→ `parent_name` à l'index). Un récepteur Go générique (`*Pile[K]`,
//! `Pile[K]`) nomme son type sans les crochets ; une méthode Python a sa classe pour parent, une classe
//! imbriquée la sienne.

use std::collections::HashMap;

use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};

fn parents(fichier: &str, src: &str) -> Vec<(String, Option<String>)> {
    let chemin = format!("/virtual/{fichier}");
    let a = ProjectParser::new(ProjectParserOptions { verbose: false }).parse_project(ParseProjectOptions {
        root: "/virtual".to_string(),
        files: vec![chemin.clone()],
        content_map: Some(HashMap::from([(chemin, src.to_string())])),
        resolve_relationships: Some(false),
        resolver_options: None,
    });
    let mut v: Vec<(String, Option<String>)> = a.files.values().flat_map(|fa| fa.scopes.iter()).map(|s| (s.name.clone(), s.parent.clone())).collect();
    v.sort();
    v
}

#[test]
fn go_un_recepteur_generique_nomme_son_type() {
    let src = "package pile\n\ntype Pile[K any] struct {\n\telements []K\n}\n\nfunc (p *Pile[K]) Push(k K) {\n\tp.elements = append(p.elements, k)\n}\n\nfunc (p Pile[K]) Len() int {\n\treturn len(p.elements)\n}\n\nfunc (Pile[K]) Vide() bool {\n\treturn false\n}\n\nfunc (s *Simple) Nom() string {\n\treturn \"\"\n}\n\ntype Simple struct{}\n";
    let p = parents("pile.go", src);
    for m in ["Push", "Len", "Vide"] {
        assert!(p.contains(&(m.to_string(), Some("Pile".to_string()))), "{m} appartient à Pile : {p:?}");
    }
    assert!(p.contains(&("Nom".to_string(), Some("Simple".to_string()))), "{p:?}");
}

#[test]
fn python_une_methode_a_sa_classe_pour_parent() {
    let src = "class Lecteur:\n    def lire(self):\n        return 1\n\n    class Tampon:\n        def vider(self):\n            return 0\n\n\ndef seule():\n    return 2\n";
    let p = parents("lecteur.py", src);
    assert!(p.contains(&("lire".to_string(), Some("Lecteur".to_string()))), "{p:?}");
    assert!(p.contains(&("vider".to_string(), Some("Tampon".to_string()))), "une classe imbriquée nomme la sienne : {p:?}");
    assert!(p.contains(&("seule".to_string(), None)), "{p:?}");
}
