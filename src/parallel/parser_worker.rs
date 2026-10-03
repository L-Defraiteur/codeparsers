use crate::scope_extraction::base_scope_extraction_parser::BaseScopeExtractionParser;
use crate::scope_extraction::c_scope_extraction_parser::CScopeExtractionParser;
use crate::scope_extraction::c_sharp_scope_extraction_parser::CSharpScopeExtractionParser;
use crate::scope_extraction::cpp_scope_extraction_parser::CppScopeExtractionParser;
use crate::scope_extraction::go_scope_extraction_parser::GoScopeExtractionParser;
use crate::scope_extraction::python_scope_extraction_parser::PythonScopeExtractionParser;
use crate::scope_extraction::rust_scope_extraction_parser::RustScopeExtractionParser;
use crate::scope_extraction::types::{ScopeFileAnalysis, ScopeInfo, ScopeInfoType};

use std::cell::RefCell;

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum SupportedLanguage {
    Typescript,
    Javascript,
    Python,
    Rust,
    Go,
    C,
    Cpp,
    Csharp,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ParseFileTask {
    pub file_path: String,
    pub content: String,
    pub language: SupportedLanguage,
}

/// Per-thread parser cache. Each Rayon worker thread gets its own set of parsers,
/// created on first use and reused for subsequent files.
struct ParserCache {
    typescript: Option<BaseScopeExtractionParser>,
    python: Option<PythonScopeExtractionParser>,
    rust: Option<RustScopeExtractionParser>,
    go: Option<GoScopeExtractionParser>,
    c: Option<CScopeExtractionParser>,
    cpp: Option<CppScopeExtractionParser>,
    csharp: Option<CSharpScopeExtractionParser>,
}

impl ParserCache {
    fn new() -> Self {
        Self {
            typescript: None,
            python: None,
            rust: None,
            go: None,
            c: None,
            cpp: None,
            csharp: None,
        }
    }
}

thread_local! {
    static CACHE: RefCell<ParserCache> = RefCell::new(ParserCache::new());
}

/// Parse a single file with the appropriate language parser.
/// Parsers are cached per-thread via thread_local (one per Rayon worker).
pub fn parse_file(task: &ParseFileTask) -> ScopeFileAnalysis {
    let mut analysis = parse_file_raw(task);
    finalize(&mut analysis, &task.content);
    analysis
}

/// **Un fichier sans grammaire, rendu entier plutôt qu'ignoré.**
///
/// Ni un échec ni une extraction : un seul scope [`ScopeInfoType::TexteBrut`]
/// couvrant tout le fichier, `aucun_parseur` à vrai, et `ast_valid` à vrai —
/// on n'a pas échoué à parser, on n'a pas essayé, et la différence est réelle.
///
/// codeparsers ne décide pas si ce fichier mérite d'entrer dans un index : il
/// rend les faits — les octets, le genre — et laisse la politique au
/// consommateur. Un parseur qui saute un fichier en silence ment par omission,
/// et c'est le défaut que ce dépôt passe ses journées à débusquer.
pub fn analyser_texte_brut(file_path: &str, content: &str) -> ScopeFileAnalysis {
    let lignes = content.lines().count().max(1);
    let nom = std::path::Path::new(file_path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| file_path.to_string());
    let scope = ScopeInfo {
        test: None,
        // Un scope sans nom ne se cite pas : celui-ci porte son fichier et son
        // étendue, comme les passages du cahier des charges.
        name: format!("{nom}:1-{lignes}"),
        r#type: ScopeInfoType::TexteBrut,
        scope_start_line: 1,
        scope_end_line: lignes,
        scope_start_byte: 0,
        scope_end_byte: content.len(),
        signature_start_line: 1,
        signature_end_line: 1,
        body_start_line: None,
        body_end_line: None,
        file_path: file_path.to_string(),
        // Pas de signature. Il n'y en a pas, et en inventer une serait mentir.
        signature: String::new(),
        parameters: Vec::new(),
        return_type: None,
        return_type_info: None,
        modifiers: Vec::new(),
        generic_parameters: None,
        heritage_clauses: None,
        decorator_details: None,
        content: content.to_string(),
        content_dedented: content.to_string(),
        children: Vec::new(),
        members: None,
        enum_members: None,
        variables: None,
        dependencies: Vec::new(),
        exports: Vec::new(),
        imports: Vec::new(),
        import_references: Vec::new(),
        identifier_references: Vec::new(),
        ast_valid: true,
        ast_issues: Vec::new(),
        ast_notes: Vec::new(),
        complexity: 1,
        lines_of_code: lignes,
        parent: None,
        depth: 0,
        docstring: None,
        decorators: None,
        value: None,
    };
    let mut analyse = ScopeFileAnalysis {
        file_path: file_path.to_string(),
        scopes: vec![scope],
        total_lines: lignes,
        total_scopes: 1,
        ast_valid: true,
        aucun_parseur: true,
        ..Default::default()
    };
    finalize(&mut analyse, content);
    // **`finalize` dérive les octets des lignes, et une fin de ligne exclut le
    // `\n`.** Le dernier saut de ligne d'un fichier n'est donc jamais couvert —
    // un octet perdu par fichier, et c'est une part des trous que
    // `examples/couverture.rs` mesure. Ici on connaît la vérité, on la pose.
    //
    // Le défaut général appartient au chantier de couverture : tant que les
    // offsets se dérivent des lignes, l'invariant « l'union des scopes couvre
    // le fichier » sera faux d'un octet partout.
    analyse.scopes[0].scope_start_byte = 0;
    analyse.scopes[0].scope_end_byte = content.len();
    analyse
}

/// Ce que les parseurs de langage ne remplissent pas et que tout consommateur
/// attend : le hash de contenu (blake3, le même que les UUID), les octets du
/// fichier, les offsets d'octets de chaque scope, dérivés de ses lignes, et
/// la marque de test de chaque scope.
pub fn finalize(analysis: &mut ScopeFileAnalysis, content: &str) {
    analysis.content_hash = Some(crate::utils::hash::content_hash(content));
    analysis.octets = content.len();
    // line_starts[i] = offset du premier octet de la ligne i+1 (1-based)
    let mut line_starts: Vec<usize> = vec![0];
    for (i, b) in content.bytes().enumerate() {
        if b == b'\n' {
            line_starts.push(i + 1);
        }
    }
    let end_of_line = |line: usize| -> usize {
        // fin de la ligne `line` (1-based), '\n' exclu
        match line_starts.get(line) {
            Some(&next) => next.saturating_sub(1),
            None => content.len(),
        }
    };
    for scope in &mut analysis.scopes {
        let start = scope.scope_start_line.max(1);
        let end = scope.scope_end_line.max(start);
        scope.scope_start_byte = line_starts.get(start - 1).copied().unwrap_or(content.len());
        scope.scope_end_byte = end_of_line(end).max(scope.scope_start_byte);
    }
    let chemin = analysis.file_path.clone();
    crate::scope_extraction::test_marks::mark_tests(&mut analysis.scopes, content, &chemin);
    attach_import_origins(analysis);
    ordre_fixe(analysis);
}

/// **Une sortie qui ne dépend pas de l'exécution.** Des `HashSet` et
/// `HashMap` remplissent les références et les imports ; leur ordre change
/// d'un fil à l'autre (chaque fil tire ses propres clés de hachage) : le
/// même fichier rendait jusqu'à 28 ordres différents sur 64 analyses en
/// parallèle, et un consommateur qui prend le premier site d'un usage
/// changeait de ligne d'une indexation à l'autre. Triés ici, une fois.
fn ordre_fixe(analysis: &mut ScopeFileAnalysis) {
    for scope in &mut analysis.scopes {
        scope.identifier_references.sort_by(|a, b| {
            (a.line, a.column, &a.identifier, &a.qualifier, &a.context).cmp(&(b.line, b.column, &b.identifier, &b.qualifier, &b.context))
        });
        scope.import_references.sort_by(|a, b| (a.line, &a.source, &a.imported, &a.alias).cmp(&(b.line, &b.source, &b.imported, &b.alias)));
    }
    analysis.import_references.sort_by(|a, b| (a.line, &a.source, &a.imported, &a.alias).cmp(&(b.line, &b.source, &b.imported, &b.alias)));
    analysis.imports.sort();
    analysis.dependencies.sort();
    analysis.exports.sort();
}

/// **L'import de chaque référence** : le nom visible (l'alias, sinon le nom
/// importé) → `(module, nom d'origine)`, depuis les imports du fichier et
/// ceux que les scopes portent. Un nom importé de deux endroits différents
/// n'a pas d'origine : on ne devine pas. Une référence dont le nom n'est pas
/// importé prend l'origine de la tête de son qualificatif (`a::b::f` → `a`).
fn attach_import_origins(analysis: &mut ScopeFileAnalysis) {
    use crate::scope_extraction::types::ImportOrigin;
    use std::collections::HashMap;
    let mut table: HashMap<String, Option<(String, String)>> = HashMap::new();
    let tous = analysis.import_references.iter().chain(analysis.scopes.iter().flat_map(|s| s.import_references.iter()));
    for imp in tous {
        let visible = imp.alias.clone().unwrap_or_else(|| imp.imported.clone());
        if visible.is_empty() || visible == "*" {
            continue;
        }
        // Le chemin entier quand il est gardé à part (Rust), et alors le
        // nom seul de l'élément (`imported` y garde `collections::HashMap`).
        let origine = match &imp.module_path {
            Some(m) => (m.clone(), imp.imported.rsplit("::").next().unwrap_or(&imp.imported).to_string()),
            None => (imp.source.clone(), imp.imported.clone()),
        };
        table
            .entry(visible)
            .and_modify(|o| {
                if o.as_ref() != Some(&origine) {
                    *o = None;
                }
            })
            .or_insert(Some(origine));
    }
    if table.is_empty() {
        return;
    }
    let lire = |nom: &str| table.get(nom).cloned().flatten();
    for scope in &mut analysis.scopes {
        for r in &mut scope.identifier_references {
            if let Some((source, imported)) = lire(&r.identifier) {
                r.import_origin = Some(ImportOrigin { source, imported, via_qualifier: false });
            } else if let Some(q) = r.qualifier.as_deref() {
                let tete = q.split("::").next().unwrap_or(q).split('.').next().unwrap_or(q);
                if let Some((source, imported)) = lire(tete) {
                    r.import_origin = Some(ImportOrigin { source, imported, via_qualifier: true });
                }
            }
        }
    }
}

fn parse_file_raw(task: &ParseFileTask) -> ScopeFileAnalysis {
    CACHE.with(|cell| {
        let mut cache = cell.borrow_mut();
        match task.language {
            SupportedLanguage::Typescript | SupportedLanguage::Javascript => {
                let parser = cache.typescript.get_or_insert_with(|| {
                    let p = BaseScopeExtractionParser::new(SupportedLanguage::Typescript);
                    p.initialize();
                    p
                });
                parser.parse_file(&task.file_path, &task.content, None)
            }
            SupportedLanguage::Python => {
                let parser = cache.python.get_or_insert_with(|| {
                    let p = PythonScopeExtractionParser::new(SupportedLanguage::Python);
                    p.initialize();
                    p
                });
                parser.parse_file(&task.file_path, &task.content)
            }
            SupportedLanguage::Rust => {
                let parser = cache.rust.get_or_insert_with(|| {
                    let p = RustScopeExtractionParser::new();
                    p.initialize();
                    p
                });
                parser.parse_file(&task.file_path, &task.content)
            }
            SupportedLanguage::Go => {
                let parser = cache.go.get_or_insert_with(|| {
                    let p = GoScopeExtractionParser::new();
                    p.initialize();
                    p
                });
                parser.parse_file(&task.file_path, &task.content)
            }
            SupportedLanguage::C => {
                let parser = cache.c.get_or_insert_with(|| {
                    let p = CScopeExtractionParser::new();
                    p.initialize();
                    p
                });
                parser.parse_file(&task.file_path, &task.content)
            }
            SupportedLanguage::Cpp => {
                let parser = cache.cpp.get_or_insert_with(|| {
                    let p = CppScopeExtractionParser::new();
                    p.initialize();
                    p
                });
                parser.parse_file(&task.file_path, &task.content)
            }
            SupportedLanguage::Csharp => {
                let parser = cache.csharp.get_or_insert_with(|| {
                    let p = CSharpScopeExtractionParser::new();
                    p.initialize();
                    p
                });
                parser.parse_file(&task.file_path, &task.content)
            }
        }
    })
}
