//! La coloration du code par les grammaires tree-sitter (7 novembre 2026).
//!
//! Chaque crate de grammaire livre sa requête `highlights.scm` ; le moteur officiel
//! (`tree-sitter-highlight`) l'applique. Les noms de capture (`@function.method`, `@type.builtin`…)
//! se ramènent à une liste fermée de [`Sorte`]s par leur premier segment, un nom inconnu tombe dans
//! [`Sorte::Other`]. Conception : `extension/rag3weaver/docs/10-octobre-2026-18h20/codeparsers/
//! 12-coloration-par-les-grammaires.md` (rag3db).
//!
//! Remplace l'ancien `SyntaxHighlightingParser` (portage de février 2026), qui classait les nœuds à
//! la main pour TypeScript et Python seulement, et que rien n'appelait.

use std::collections::HashMap;
use std::sync::OnceLock;

use tree_sitter_highlight::{HighlightConfiguration, HighlightEvent, Highlighter};

/// La sorte d'un span : la classe `hl-<sorte>` de la page et de tty.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Sorte {
    Keyword,
    Type,
    Function,
    String,
    Comment,
    Number,
    Constant,
    Variable,
    Property,
    Operator,
    Punctuation,
    Attribute,
    Label,
    Module,
    Other,
}

impl Sorte {
    /// Toutes les sortes, dans l'ordre de la liste fermée (à dire dans une réponse).
    pub const TOUTES: [Sorte; 15] = [
        Sorte::Keyword, Sorte::Type, Sorte::Function, Sorte::String, Sorte::Comment,
        Sorte::Number, Sorte::Constant, Sorte::Variable, Sorte::Property, Sorte::Operator,
        Sorte::Punctuation, Sorte::Attribute, Sorte::Label, Sorte::Module, Sorte::Other,
    ];

    pub fn nom(self) -> &'static str {
        match self {
            Sorte::Keyword => "keyword",
            Sorte::Type => "type",
            Sorte::Function => "function",
            Sorte::String => "string",
            Sorte::Comment => "comment",
            Sorte::Number => "number",
            Sorte::Constant => "constant",
            Sorte::Variable => "variable",
            Sorte::Property => "property",
            Sorte::Operator => "operator",
            Sorte::Punctuation => "punctuation",
            Sorte::Attribute => "attribute",
            Sorte::Label => "label",
            Sorte::Module => "module",
            Sorte::Other => "other",
        }
    }

    /// La sorte d'un nom de capture, par son premier segment.
    pub fn de_capture(nom: &str) -> Sorte {
        let tete = nom.split('.').next().unwrap_or(nom);
        Sorte::TOUTES
            .iter()
            .copied()
            .find(|s| *s != Sorte::Other && s.nom() == tete)
            .unwrap_or(Sorte::Other)
    }
}

/// Un span coloré : `[debut, fin)` en octets du texte UTF-8.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Span {
    pub debut: usize,
    pub fin: usize,
    pub sorte: Sorte,
}

/// Les langues colorées, par le nom qu'on leur donne (celui des fences Markdown).
pub fn langues() -> &'static [&'static str] {
    &[
        "rust", "python", "go", "c", "cpp", "csharp", "javascript", "jsx", "typescript", "tsx",
        "bash", "css", "html",
    ]
}

/// Ramène un nom de langue courant (`rs`, `c++`, `ts`, `py`, `sh`…) au nom de [`langues`].
pub fn nom_de_langue(langue: &str) -> Option<&'static str> {
    let l = langue.trim().to_ascii_lowercase();
    Some(match l.as_str() {
        "rust" | "rs" => "rust",
        "python" | "py" => "python",
        "go" | "golang" => "go",
        "c" | "h" => "c",
        "cpp" | "c++" | "cc" | "cxx" | "hpp" | "hh" => "cpp",
        "csharp" | "c#" | "cs" => "csharp",
        "javascript" | "js" | "mjs" | "cjs" => "javascript",
        "jsx" => "jsx",
        "typescript" | "ts" | "mts" | "cts" => "typescript",
        "tsx" => "tsx",
        "bash" | "sh" | "shell" | "zsh" => "bash",
        "css" => "css",
        "html" | "htm" => "html",
        _ => return None,
    })
}

fn configuration(nom: &str) -> Option<HighlightConfiguration> {
    let js = tree_sitter_javascript::HIGHLIGHT_QUERY;
    let jsx = tree_sitter_javascript::JSX_HIGHLIGHT_QUERY;
    let ts = tree_sitter_typescript::HIGHLIGHTS_QUERY;
    // Les requêtes qui héritent se lisent après leur parente, dans l'ordre des `tree-sitter.json`
    // amont : C puis C++, JavaScript (et JSX) puis TypeScript.
    let (langage, requete): (tree_sitter::Language, String) = match nom {
        "rust" => (tree_sitter_rust::LANGUAGE.into(), tree_sitter_rust::HIGHLIGHTS_QUERY.into()),
        "python" => (tree_sitter_python::LANGUAGE.into(), tree_sitter_python::HIGHLIGHTS_QUERY.into()),
        "go" => (tree_sitter_go::LANGUAGE.into(), tree_sitter_go::HIGHLIGHTS_QUERY.into()),
        "c" => (tree_sitter_c::LANGUAGE.into(), tree_sitter_c::HIGHLIGHT_QUERY.into()),
        "cpp" => (
            tree_sitter_cpp::LANGUAGE.into(),
            format!("{}\n{}", tree_sitter_c::HIGHLIGHT_QUERY, tree_sitter_cpp::HIGHLIGHT_QUERY),
        ),
        "csharp" => (tree_sitter_c_sharp::LANGUAGE.into(), tree_sitter_c_sharp::HIGHLIGHTS_QUERY.into()),
        "javascript" | "jsx" => (tree_sitter_javascript::LANGUAGE.into(), format!("{jsx}\n{js}")),
        "typescript" => (tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(), format!("{js}\n{ts}")),
        "tsx" => (tree_sitter_typescript::LANGUAGE_TSX.into(), format!("{jsx}\n{js}\n{ts}")),
        "bash" => (tree_sitter_bash::LANGUAGE.into(), tree_sitter_bash::HIGHLIGHT_QUERY.into()),
        "css" => (tree_sitter_css::LANGUAGE.into(), tree_sitter_css::HIGHLIGHTS_QUERY.into()),
        "html" => (tree_sitter_html::LANGUAGE.into(), tree_sitter_html::HIGHLIGHTS_QUERY.into()),
        _ => return None,
    };
    let mut config = HighlightConfiguration::new(langage, nom, &requete, "", "")
        .unwrap_or_else(|e| panic!("la requête de coloration de {nom} ne compile pas : {e:?}"));
    let noms: Vec<String> = config.names().iter().map(|n| n.to_string()).collect();
    config.configure(&noms);
    Some(config)
}

struct Configurations {
    par_langue: HashMap<&'static str, (HighlightConfiguration, Vec<Sorte>)>,
}

fn configurations() -> &'static Configurations {
    static C: OnceLock<Configurations> = OnceLock::new();
    C.get_or_init(|| Configurations {
        par_langue: langues()
            .iter()
            .filter_map(|nom| {
                let config = configuration(nom)?;
                let sortes = config.names().iter().map(|n| Sorte::de_capture(n)).collect();
                Some((*nom, (config, sortes)))
            })
            .collect(),
    })
}

/// Colore `texte` dans `langue` (un nom de [`langues`] ou un alias de [`nom_de_langue`]).
///
/// Rend les spans colorés, triés et disjoints, en octets ; les spans contigus d'une même sorte sont
/// fondus. Une langue inconnue rend `None` (la page retombe sur son repli). Un texte qui ne se lit
/// pas en entier se colore là où l'arbre tient.
pub fn highlight(texte: &str, langue: &str) -> Option<Vec<Span>> {
    let nom = nom_de_langue(langue)?;
    let (config, sortes) = configurations().par_langue.get(nom)?;
    let mut surligneur = Highlighter::new();
    let evenements = surligneur.highlight(config, texte.as_bytes(), None, None, |_| None).ok()?;
    let mut pile: Vec<Sorte> = Vec::new();
    let mut spans: Vec<Span> = Vec::new();
    for evenement in evenements {
        match evenement.ok()? {
            HighlightEvent::HighlightStart(h) => pile.push(sortes[h.0]),
            HighlightEvent::HighlightEnd => {
                pile.pop();
            }
            HighlightEvent::Source { start, end } => {
                let Some(&sorte) = pile.last() else { continue };
                if start == end {
                    continue;
                }
                match spans.last_mut() {
                    Some(dernier) if dernier.fin == start && dernier.sorte == sorte => dernier.fin = end,
                    _ => spans.push(Span { debut: start, fin: end, sorte }),
                }
            }
        }
    }
    Some(spans)
}
