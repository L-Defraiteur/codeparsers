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

/// **Le type des variables d'un scope, quand il se lit sans inférence.**
///
/// Trois sources seulement : (a) l'annotation d'une variable, (b) le type
/// d'un paramètre, (c) un initialiseur constructeur — littéral de struct
/// (`Node { … }`), fonction associée d'un type (`Node::new(…)`), `new Node()`,
/// ou appel d'une classe en Python (`Node()`). Aucune inférence à travers un
/// appel : `let n = make();` ne dit rien. Un nom lié à deux types différents
/// dans le même scope est écarté plutôt que deviné.
pub fn typed_bindings(noeud: Node, content: &str) -> std::collections::HashMap<String, String> {
    let mut vus: std::collections::HashMap<String, Option<String>> = std::collections::HashMap::new();
    collect_typed(noeud, content, &mut vus);
    vus.into_iter().filter_map(|(n, t)| t.map(|t| (n, t))).collect()
}

fn texte<'a>(n: Node, content: &'a str) -> &'a str {
    content.get(n.start_byte()..n.end_byte()).unwrap_or("")
}

fn collect_typed(n: Node, content: &str, vus: &mut std::collections::HashMap<String, Option<String>>) {
    let k = n.kind();
    let liaison = match k {
        "let_declaration" => Some((n.child_by_field_name("pattern"), n.child_by_field_name("type"), n.child_by_field_name("value"))),
        "parameter" | "required_parameter" | "optional_parameter" => {
            Some((n.child_by_field_name("pattern").or_else(|| n.child_by_field_name("name")), n.child_by_field_name("type"), None))
        }
        "variable_declarator" => Some((n.child_by_field_name("name"), n.child_by_field_name("type"), n.child_by_field_name("value"))),
        "typed_parameter" => Some((n.named_child(0), n.child_by_field_name("type"), None)),
        "typed_default_parameter" => Some((n.child_by_field_name("name"), n.child_by_field_name("type"), n.child_by_field_name("value"))),
        "assignment" => Some((n.child_by_field_name("left"), n.child_by_field_name("type"), n.child_by_field_name("right"))),
        "parameter_declaration" | "declaration" => {
            let decl = n.child_by_field_name("declarator");
            let (nom, valeur) = match decl {
                Some(d) if d.kind() == "init_declarator" => (d.child_by_field_name("declarator"), d.child_by_field_name("value")),
                d => (d, None),
            };
            Some((nom, n.child_by_field_name("type"), valeur))
        }
        _ => None,
    };
    if let Some((nom, annotation, valeur)) = liaison {
        if let Some(nom) = nom.and_then(|x| simple_name(x, content)) {
            let type_ = annotation
                .and_then(|t| base_type_name(texte(t, content)))
                .or_else(|| valeur.and_then(|v| initializer_type(v, content)))
                .and_then(|t| if t == "Self" { enclosing_impl_type(n, content) } else { Some(t) });
            if let Some(t) = type_ {
                match vus.get(&nom) {
                    Some(Some(ancien)) if *ancien != t => {
                        vus.insert(nom, None);
                    }
                    Some(None) => {}
                    _ => {
                        vus.insert(nom, Some(t));
                    }
                }
            }
        }
    }
    let mut c = n.walk();
    for enfant in n.named_children(&mut c) {
        collect_typed(enfant, content, vus);
    }
}

/// `Self` désigne le type de l'`impl` qui contient le nœud.
fn enclosing_impl_type(n: Node, content: &str) -> Option<String> {
    let mut a = n.parent();
    while let Some(p) = a {
        if p.kind() == "impl_item" {
            return p.child_by_field_name("type").and_then(|t| base_type_name(texte(t, content)));
        }
        a = p.parent();
    }
    None
}

/// Le nom qu'un motif ou un déclarateur lie, s'il en lie un seul.
fn simple_name(n: Node, content: &str) -> Option<String> {
    match n.kind() {
        "identifier" => {
            let t = texte(n, content);
            (t != "self" && !t.is_empty()).then(|| t.to_string())
        }
        "mut_pattern" | "reference_pattern" | "reference_declarator" | "pointer_declarator" => {
            let mut c = n.walk();
            let enfants: Vec<Node> = n.named_children(&mut c).collect();
            enfants.into_iter().rev().find_map(|e| simple_name(e, content))
        }
        _ => None,
    }
}

/// Le type que construit un initialiseur, quand il se lit sur lui.
fn initializer_type(v: Node, content: &str) -> Option<String> {
    match v.kind() {
        "struct_expression" => v.child_by_field_name("name").and_then(|t| base_type_name(texte(t, content))),
        "new_expression" => v
            .child_by_field_name("constructor")
            .or_else(|| v.child_by_field_name("type"))
            .and_then(|t| base_type_name(texte(t, content))),
        "call_expression" => {
            let f = v.child_by_field_name("function")?;
            if f.kind() != "scoped_identifier" {
                return None;
            }
            let chemin = base_type_name(texte(f.child_by_field_name("path")?, content))?;
            let fonction = texte(f.child_by_field_name("name")?, content);
            let constructeur = matches!(fonction, "new" | "default") || fonction.starts_with("new_") || fonction.starts_with("with_") || fonction.starts_with("from");
            (constructeur && chemin.starts_with(|c: char| c.is_uppercase())).then_some(chemin)
        }
        "call" => {
            let f = v.child_by_field_name("function")?;
            let nom = texte(f, content);
            (f.kind() == "identifier" && nom.starts_with(|c: char| c.is_uppercase())).then(|| nom.to_string())
        }
        _ => None,
    }
}

/// Le nom du type que désigne un texte de type : `&mut Node` → `Node`,
/// `crate::a::Node<T>` → `Node`, `: Node | null` → `Node`. `Box`, `Arc` et
/// `Rc` se déballent d'un niveau (on appelle à travers eux les méthodes du
/// type enveloppé) ; tout autre générique garde son nom (`Option<Node>` est
/// une `Option`).
pub fn base_type_name(texte: &str) -> Option<String> {
    let mut t = texte.trim().trim_start_matches(':').trim();
    loop {
        let avant = t;
        for prefixe in ["&", "*", "mut ", "const ", "dyn ", "impl ", "readonly "] {
            t = t.trim_start_matches(prefixe).trim_start();
        }
        if t == avant {
            break;
        }
    }
    let chemin: String = t.chars().take_while(|c| c.is_alphanumeric() || *c == '_' || *c == ':' || *c == '.').collect();
    let nom = chemin.rsplit(|c| c == ':' || c == '.').find(|s| !s.is_empty())?.to_string();
    if matches!(nom.as_str(), "Box" | "Arc" | "Rc") {
        let reste = &t[chemin.len()..];
        if let Some(interieur) = reste.strip_prefix('<').and_then(|r| r.strip_suffix('>')) {
            return base_type_name(interieur);
        }
    }
    (!nom.is_empty() && nom.starts_with(|c: char| c.is_alphabetic() || c == '_') && !matches!(nom.as_str(), "auto" | "var" | "let" | "const")).then_some(nom)
}
