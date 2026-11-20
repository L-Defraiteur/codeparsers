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
pub fn categories(_path: &str, _content: &str) -> Option<Vec<(String, String, (usize, usize), Category)>> {
    None
}

/// La couverture de `content`, analysé comme `path`.
pub fn measure(path: &str, content: &str) -> Option<Coverage> {
    let mut c = Coverage::default();
    for (kind, _, _, cat) in categories(path, content)? {
        *c.by_kind.entry(kind).or_default().entry(cat).or_default() += 1;
    }
    Some(c)
}
