//! **Le genre d'un usage, lu sur l'AST.**
//!
//! Une seule fonction pour toutes les grammaires : elle ne regarde que les
//! noms de nœuds et de champs, que tree-sitter partage largement d'un langage
//! à l'autre (`call_expression` et son champ `function`, le champ `type` d'une
//! déclaration…). Ce qui n'est ni un appel, ni un type, ni un import, ni un
//! héritage est `Other` : une lecture, une écriture, un argument.

use super::types::UsageKind;
use tree_sitter::Node;

/// Accès à un membre : l'identifiant visé en est le dernier enfant nommé
/// (`a.b`, `a::b`, `a->b`, `pkg.F`, `obj.attr`).
const ACCES: &[&str] = &[
    "member_expression",
    "field_expression",
    "scoped_identifier",
    "qualified_identifier",
    "member_access_expression",
    "selector_expression",
    "attribute",
    "scoped_type_identifier",
    "qualified_name",
    "qualified_type",
];

/// Un appel, ou une instanciation : le nœud, et le champ qui porte ce qu'on
/// appelle.
const APPELS: &[(&str, &str)] = &[
    ("call_expression", "function"),
    ("call", "function"),
    ("invocation_expression", "function"),
    ("macro_invocation", "macro"),
    ("new_expression", "constructor"),
    ("new_expression", "type"),
    ("object_creation_expression", "type"),
    ("struct_expression", "name"),
    ("composite_literal", "type"),
];

/// Une clause d'héritage ou d'implémentation, quel que soit le niveau.
const HERITAGE: &[&str] = &[
    "class_heritage",
    "extends_clause",
    "extends_type_clause",
    "implements_clause",
    "class_implements_clause",
    "base_class_clause",
    "base_list",
];

/// Une instruction d'import, quel que soit le niveau.
const IMPORTS: &[&str] = &[
    "import_statement",
    "import_from_statement",
    "future_import_statement",
    "import_declaration",
    "import_spec",
    "use_declaration",
    "using_directive",
    "preproc_include",
];

/// Une position de type par le nom du nœud parent.
const POSITIONS_DE_TYPE: &[&str] = &[
    "type",
    "type_annotation",
    "type_arguments",
    "type_argument_list",
    "generic_type",
    "array_type",
    "pointer_type",
    "reference_type",
    "nullable_type",
    "optional_type",
    "slice_type",
    "map_type",
    "trait_bounds",
    "constraint",
];

/// Nœuds qui sont eux-mêmes un nom de type.
const NOMS_DE_TYPE: &[&str] = &[
    "type_identifier",
    "scoped_type_identifier",
    "generic_type",
    "qualified_type",
    "predefined_type",
];

fn est_le_champ(parent: Node, champ: &str, n: Node) -> bool {
    parent.child_by_field_name(champ).is_some_and(|c| c.id() == n.id())
}

/// Le genre de l'usage que fait le nœud `noeud` (un identifiant) de ce qu'il
/// nomme.
pub fn usage_of(noeud: Node) -> UsageKind {
    // Héritage et import se lisent sur les ancêtres : `extends a.B<T>` ou
    // `use crate::a::B` placent l'identifiant plusieurs niveaux plus bas.
    let mut a = noeud.parent();
    while let Some(p) = a {
        let k = p.kind();
        if HERITAGE.contains(&k) {
            return UsageKind::Inheritance;
        }
        if IMPORTS.contains(&k) {
            return UsageKind::Import;
        }
        // Python : `class C(Base)`, la liste des superclasses.
        if k == "argument_list" && p.parent().is_some_and(|g| g.kind() == "class_definition" && est_le_champ(g, "superclasses", p)) {
            return UsageKind::Inheritance;
        }
        // Rust : `impl Trait for X`, le trait.
        if k == "impl_item" {
            let mut n = noeud;
            while let Some(q) = n.parent() {
                if q.id() == p.id() {
                    break;
                }
                n = q;
            }
            if est_le_champ(p, "trait", n) {
                return UsageKind::Inheritance;
            }
            break;
        }
        // Les corps et les déclarations arrêtent la remontée : rien au-dessus
        // ne dit plus comment cet identifiant-ci est utilisé.
        if k.ends_with("block") || k.ends_with("body") || k.ends_with("_statement") || k.ends_with("declaration_list")
            || k.ends_with("_definition") || k.ends_with("_declaration") || k.ends_with("_item") || k == "program" || k == "module"
            || k == "source_file" || k == "translation_unit" || k == "compilation_unit"
        {
            break;
        }
        a = p.parent();
    }

    // Remonter la chaîne d'accès tant qu'on en est le dernier segment :
    // dans `a.b.c()`, c'est `a.b.c` qui est appelé.
    let mut n = noeud;
    while let Some(p) = n.parent() {
        let k = p.kind();
        let dernier = p.named_child(p.named_child_count().saturating_sub(1)).is_some_and(|c| c.id() == n.id());
        if ACCES.contains(&k) && dernier {
            n = p;
        } else if (k == "generic_function" && est_le_champ(p, "function", n))
            || (k == "generic_name" && p.named_child(0).is_some_and(|c| c.id() == n.id()))
        {
            // `take_or_clone::<T>(…)`, `Foo<int>()` : l'appel porte sur le nom générique.
            n = p;
        } else {
            break;
        }
    }

    if let Some(p) = n.parent() {
        let k = p.kind();
        // Dans les arguments d'une macro Rust (`write!(f, "{}", self.f())`), il
        // n'y a que des jetons : un nom suivi d'une parenthèse y est un appel.
        if k == "token_tree" && n.next_sibling().is_some_and(|s| s.kind() == "token_tree" && s.child(0).is_some_and(|c| c.kind() == "(")) {
            return UsageKind::Call;
        }
        if APPELS.iter().any(|(nk, champ)| *nk == k && est_le_champ(p, champ, n)) {
            return UsageKind::Call;
        }
        if POSITIONS_DE_TYPE.contains(&k) || est_le_champ(p, "type", n) || est_le_champ(p, "return_type", n) {
            return UsageKind::Type;
        }
    }
    if NOMS_DE_TYPE.contains(&n.kind()) {
        return UsageKind::Type;
    }
    UsageKind::Other
}

/// Le genre d'un usage trouvé dans le texte, sans AST : seule une ligne
/// d'import se reconnaît sûrement ; le reste est `Other`.
pub fn usage_of_line(ligne: &str) -> UsageKind {
    let l = ligne.trim_start();
    let import = ["import ", "from ", "use ", "pub use ", "#include", "using ", "require("]
        .iter()
        .any(|p| l.starts_with(p));
    if import { UsageKind::Import } else { UsageKind::Other }
}
