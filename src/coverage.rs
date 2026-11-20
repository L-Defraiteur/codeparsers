//! **La couverture de l'analyse, mesurée** (lot 1 de l'intra-scope, 20 novembre 2026, oui de Lucie) : chaque nœud
//! nommé de l'arbre tree-sitter d'un fichier tombe dans une catégorie, selon ce que codeparsers en fait — une
//! référence relevée à sa position, le nom d'un scope, un identifiant écarté (locale, paramètre, mot vide, nom
//! intégré), un nœud neutre (littéral, commentaire), ou une structure (tout autre nœud). La mesure ne change rien au
//! graphe : elle dit, par langue et par sorte de nœud, où l'analyse s'arrête.

use std::collections::BTreeMap;

/// Les catégories d'un nœud nommé.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize)]
pub enum Category {
    /// Un identifiant que codeparsers relève comme référence, à sa ligne et à sa colonne.
    Reference,
    /// Le nom d'un scope (le champ `name` d'une définition).
    Declaration,
    /// Un autre identifiant : une locale, un paramètre, un mot vide ou un nom intégré, que l'analyse écarte.
    Excluded,
    /// Un littéral ou un commentaire.
    Neutral,
    /// Tout autre nœud nommé (instruction, expression, type…).
    Structure,
}

/// La couverture d'un fichier : par sorte de nœud, le nombre de nœuds de chaque catégorie.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct Coverage {
    pub by_kind: BTreeMap<String, BTreeMap<Category, usize>>,
}

impl Coverage {
    /// Le nombre de nœuds d'une catégorie, toutes sortes confondues.
    pub fn count(&self, c: Category) -> usize {
        self.by_kind.values().map(|m| m.get(&c).copied().unwrap_or(0)).sum()
    }
}

/// La catégorie de chaque nœud nommé de `content`, analysé comme `path` (la langue se lit à son extension) ;
/// `None` pour une langue que codeparsers ne lit pas. Avec, pour les identifiants, le texte et la position
/// `(ligne à partir de 1, colonne en octets)` de chacun.
pub fn categories(path: &str, content: &str) -> Option<Vec<(String, String, (usize, usize), Category)>> {
    use crate::parallel::parser_worker::SupportedLanguage as L;
    let langue = crate::parallel::project_parser::detect_language_from_path(path)?;
    let grammaire: tree_sitter::Language = match langue {
        L::Typescript | L::Javascript => {
            if crate::scope_extraction::base_scope_extraction_parser::uses_tsx_grammar(path) {
                tree_sitter_typescript::LANGUAGE_TSX.into()
            } else {
                tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()
            }
        }
        L::Python => tree_sitter_python::LANGUAGE.into(),
        L::Rust => tree_sitter_rust::LANGUAGE.into(),
        L::C => tree_sitter_c::LANGUAGE.into(),
        L::Cpp => tree_sitter_cpp::LANGUAGE.into(),
        L::Csharp => tree_sitter_c_sharp::LANGUAGE.into(),
        L::Go => tree_sitter_go::LANGUAGE.into(),
    };
    // Ce que codeparsers en fait : ses références, par position.
    let chemin = format!("/couverture/{}", path.trim_start_matches('/'));
    let analyse = crate::parallel::project_parser::ProjectParser::new(crate::parallel::project_parser::ProjectParserOptions { verbose: false })
        .parse_project(crate::parallel::project_parser::ParseProjectOptions {
            root: "/couverture".to_string(),
            files: vec![chemin.clone()],
            content_map: Some(std::collections::HashMap::from([(chemin, content.to_string())])),
            resolve_relationships: Some(false),
            resolver_options: None,
        });
    let mut references: std::collections::HashSet<(usize, usize)> = std::collections::HashSet::new();
    fn relever(s: &crate::scope_extraction::types::ScopeInfo, r: &mut std::collections::HashSet<(usize, usize)>) {
        for x in &s.identifier_references {
            if let Some(col) = x.column {
                r.insert((x.line, col));
            }
        }
        for c in &s.children {
            relever(c, r);
        }
    }
    for f in analyse.files.values() {
        for s in &f.scopes {
            relever(s, &mut references);
        }
    }
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&grammaire).ok()?;
    let arbre = parser.parse(content, None)?;
    let mut sortie = Vec::new();
    let mut curseur = arbre.walk();
    let mut fini = false;
    while !fini {
        let n = curseur.node();
        if n.is_named() {
            let sorte = n.kind();
            let texte = content.get(n.start_byte()..n.end_byte()).unwrap_or("").to_string();
            let pos = (n.start_position().row + 1, n.start_position().column);
            // Un nom est une feuille : un chemin (`scoped_identifier`, `qualified_identifier`) contient ses noms, il
            // n'en est pas un ; il compte en structure, et ses noms un à un.
            let est_un_nom = (sorte == "identifier" || sorte.ends_with("_identifier")) && n.named_child_count() == 0;
            let categorie = if est_un_nom {
                let nom_d_un_scope = n.parent().and_then(|p| p.child_by_field_name("name")).is_some_and(|x| x.id() == n.id());
                if references.contains(&pos) {
                    Category::Reference
                } else if nom_d_un_scope {
                    Category::Declaration
                } else {
                    Category::Excluded
                }
            } else if sorte.contains("comment")
                || sorte.contains("string")
                || sorte.contains("literal")
                || sorte.contains("number")
                || sorte.contains("integer")
                || sorte.contains("float")
                || matches!(sorte, "true" | "false" | "null" | "none" | "nil" | "char" | "escape_sequence")
            {
                Category::Neutral
            } else {
                Category::Structure
            };
            let garde_le_texte = est_un_nom || categorie == Category::Neutral || sorte.ends_with("identifier");
            sortie.push((sorte.to_string(), if garde_le_texte { texte } else { String::new() }, pos, categorie));
        }
        if curseur.goto_first_child() {
            continue;
        }
        while !curseur.goto_next_sibling() {
            if !curseur.goto_parent() {
                fini = true;
                break;
            }
        }
    }
    Some(sortie)
}

/// La couverture de `content`, analysé comme `path`.
pub fn measure(path: &str, content: &str) -> Option<Coverage> {
    let mut c = Coverage::default();
    for (kind, _, _, cat) in categories(path, content)? {
        *c.by_kind.entry(kind).or_default().entry(cat).or_default() += 1;
    }
    Some(c)
}
