use serde_json;

/// Types for Scope Extraction Parser

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ParameterInfo {
    pub name: String,
    pub r#type: Option<String>,
    pub optional: bool,
    pub default_value: Option<String>,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ImportReferenceKind {
    Default,
    Named,
    Namespace,
    #[serde(rename = "side-effect")]
    SideEffect,
    Dynamic,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ImportReference {
    pub source: String,
    pub imported: String,
    pub alias: Option<String>,
    pub kind: ImportReferenceKind,
    pub is_local: bool,
    pub line: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum IdentifierReferenceKind {
    Import,
    #[serde(rename = "local_scope")]
    LocalScope,
    Builtin,
    Unknown,
}

/// Comment un usage se sert de ce qu'il nomme, lu sur l'AST là où la
/// grammaire le dit : la fonction d'un appel (ou d'une instanciation), une
/// position de type, une clause d'héritage, une instruction d'import. Tout le
/// reste (lecture, écriture, argument) est `Other`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UsageKind {
    Call,
    Type,
    Import,
    Inheritance,
    Other,
}

/// Un usage localisé : son genre et sa ligne (1 = première ligne du fichier).
/// La ligne manque quand l'analyseur ne la connaît pas (import trouvé par
/// expression régulière sans position).
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct UsageSite {
    pub usage: UsageKind,
    pub line: Option<usize>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct IdentifierReference {
    pub identifier: String,
    pub line: usize,
    pub column: Option<usize>,
    pub context: Option<String>,
    pub qualifier: Option<String>,
    pub kind: Option<IdentifierReferenceKind>,
    pub source: Option<String>,
    pub target_scope: Option<String>,
    pub is_local_import: Option<bool>,
    /// Le genre de cet usage ; `None` seulement pour une référence construite
    /// hors de l'analyseur.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<UsageKind>,
    /// Le type de la variable `qualifier`, quand il se lit sans inférence
    /// (annotation, paramètre typé, initialiseur constructeur) : `node` dans
    /// `node.with_delai(t)` est un `Node`. Il permet de relier l'appel à la
    /// méthode de ce type plutôt que de l'abandonner.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub qualifier_type: Option<String>,
    /// Quand le type du qualificatif ne se lit pas sur place mais dans une
    /// déclaration ailleurs (un champ, un type de retour) : ce qu'il faut y
    /// lire. Le résolveur, qui voit tout le projet, le calcule.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub qualifier_deferred: Option<DeferredType>,
}

/// Un type à lire dans une déclaration d'ailleurs.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeferredType {
    /// Le type déclaré du champ `field` du type `owner` (`self.store`,
    /// `v.store` avec `v: Svc`, un champ implicite en C++).
    FieldOf { owner: String, field: String },
    /// Le type de retour déclaré de la fonction `function` (`let s =
    /// make_store()`, `make_store().get()`) ; `unwrap` quand l'appel est
    /// suivi de `?`, qui déballe un `Result` ou une `Option`.
    ReturnOf { function: String, unwrap: bool },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum VariableInfoKind {
    Const,
    Let,
    Var,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VariableInfo {
    pub name: String,
    pub r#type: Option<String>,
    pub kind: VariableInfoKind,
    pub line: usize,
    pub scope: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ClassMemberInfoMemberType {
    Property,
    Method,
    Getter,
    Setter,
    Constructor,
    Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ClassMemberInfoAccessibility {
    Public,
    Private,
    Protected,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ClassMemberInfo {
    pub name: String,
    pub r#type: Option<String>,
    pub member_type: ClassMemberInfoMemberType,
    pub accessibility: Option<ClassMemberInfoAccessibility>,
    pub is_static: bool,
    pub is_readonly: bool,
    pub line: usize,
    pub signature: Option<String>,
    pub value: Option<String>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ReturnTypeInfo {
    pub r#type: String,
    pub line: usize,
    pub column: usize,
}

/// Generic/type parameter information
/// Examples: <T>, <T extends Base>, <K extends keyof T>

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct GenericParameter {
    pub name: String,
    pub constraint: Option<String>,
    pub default_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum HeritageClauseClause {
    Extends,
    Implements,
}

/// Heritage clause (extends/implements)
/// Separate from signature for clean querying

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HeritageClause {
    pub clause: HeritageClauseClause,
    pub types: Vec<String>,
}

/// Decorator information with full details
/// For both TypeScript and Python decorators

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct DecoratorInfo {
    pub name: String,
    pub arguments: Option<String>,
    pub line: usize,
}

/// Enum member with value

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct EnumMemberInfo {
    pub name: String,
    pub value: Option<serde_json::Value>,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ScopeInfoType {
    Class,
    Interface,
    Function,
    Method,
    Enum,
    #[serde(rename = "type_alias")]
    TypeAlias,
    Namespace,
    Module,
    Variable,
    Lambda,
    Constant,
    Block,
    /// **Aucun parseur n'a été tenté.** Ni un échec, ni un passage qu'on n'a
    /// pas su rattacher : un fichier dont l'extension n'a pas de grammaire —
    /// un `.md`, un `.toml`, un `.sh` — rend **un** scope de ce genre,
    /// couvrant tout le fichier.
    ///
    /// La distinction avec les deux genres du cahier des charges est celle
    /// que ce dépôt tient partout ailleurs : « on n'a pas essayé » n'est pas
    /// « on a essayé et ça n'a rien donné ». Un agent qui filtre
    /// `scope_type != 'texte_brut'` cherche du code ; celui qui le garde
    /// cherche ce que quelqu'un a écrit.
    #[serde(rename = "texte_brut")]
    TexteBrut,
}

/// Le rôle d'un scope dans les tests.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TestRole {
    /// Un test : ce qu'un lanceur exécute.
    Case,
    /// Ce qui regroupe des tests (`mod tests` sous `#[cfg(test)]`, classe de
    /// tests, `describe`).
    Suite,
    /// Du code qui n'existe que pour les tests (aide, fixture).
    Support,
}

/// Ce qui dit qu'un scope est un test.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TestCertainty {
    /// La syntaxe ou l'outil le dit : `#[test]`, `#[cfg(test)]`,
    /// `func TestX(t *testing.T)` dans un `_test.go`, une sous-classe de
    /// `unittest.TestCase`, `@pytest.fixture`, `TEST(…)` de gtest.
    Certain,
    /// Seul le nom le dit, selon une convention réglable de l'outil (pytest :
    /// `test_*` dans un fichier `test_*.py`, classes `Test*`).
    Convention,
}

/// La marque de test d'un scope.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct TestMark {
    pub role: TestRole,
    pub certainty: TestCertainty,
    /// Ce qui l'a dit : `#[test]`, `#[cfg(test)]`, `@pytest.fixture`,
    /// `unittest.TestCase`, `test_*`, `TEST`, `*testing.T`…
    pub marker: String,
    /// Le nom du test quand il n'est pas celui du scope : `CalcTest.Adds`
    /// pour `TEST(CalcTest, Adds)`, dont le scope s'appelle `TEST`.
    pub name: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScopeInfo {
    pub name: String,
    pub r#type: ScopeInfoType,
    pub scope_start_line: usize,
    pub scope_end_line: usize,
    /// Offsets d'octets dans le fichier, à la granularité de la ligne : du
    /// début de `scope_start_line` à la fin de `scope_end_line` (fin de ligne
    /// exclue). Remplis par `parallel::parser_worker::finalize` après
    /// extraction, à partir des lignes — c'est ce qui fait de `File` la
    /// source de vérité des positions et permet un `read` par tranche.
    #[serde(default)]
    pub scope_start_byte: usize,
    #[serde(default)]
    pub scope_end_byte: usize,
    pub signature_start_line: usize,
    pub signature_end_line: usize,
    pub body_start_line: Option<usize>,
    pub body_end_line: Option<usize>,
    pub file_path: String,
    pub signature: String,
    pub parameters: Vec<ParameterInfo>,
    pub return_type: Option<String>,
    pub return_type_info: Option<ReturnTypeInfo>,
    pub modifiers: Vec<String>,
    pub generic_parameters: Option<Vec<GenericParameter>>,
    pub heritage_clauses: Option<Vec<HeritageClause>>,
    pub decorator_details: Option<Vec<DecoratorInfo>>,
    pub content: String,
    pub content_dedented: String,
    pub children: Vec<Box<ScopeInfo>>,
    pub members: Option<Vec<ClassMemberInfo>>,
    pub enum_members: Option<Vec<EnumMemberInfo>>,
    pub variables: Option<Vec<VariableInfo>>,
    pub dependencies: Vec<String>,
    pub exports: Vec<String>,
    pub imports: Vec<String>,
    pub import_references: Vec<ImportReference>,
    pub identifier_references: Vec<IdentifierReference>,
    pub ast_valid: bool,
    pub ast_issues: Vec<String>,
    pub ast_notes: Vec<String>,
    pub complexity: usize,
    pub lines_of_code: usize,
    pub parent: Option<String>,
    pub depth: usize,
    pub docstring: Option<String>,
    pub decorators: Option<Vec<String>>,
    pub value: Option<String>,
    /// La marque de test, posée par `parallel::parser_worker::finalize`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub test: Option<TestMark>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ScopeFileAnalysis {
    pub file_path: String,
    pub scopes: Vec<ScopeInfo>,
    pub total_lines: usize,
    pub total_scopes: usize,
    pub imports: Vec<String>,
    pub exports: Vec<String>,
    pub dependencies: Vec<String>,
    pub import_references: Vec<ImportReference>,
    pub ast_valid: bool,
    pub ast_issues: Vec<String>,
    pub content_hash: Option<String>,
    /// Octets du fichier. Rempli par `parallel::parser_worker::finalize`.
    ///
    /// Un **fait**, pas une politique : c'est au consommateur de décider si un
    /// fichier est trop gros pour son index, et de le dire. codeparsers rend
    /// toujours une analyse — il ne saute rien en silence.
    #[serde(default)]
    pub octets: usize,
    /// Vrai quand aucune grammaire n'a été tentée : le fichier est rendu en un
    /// seul scope [`ScopeInfoType::TexteBrut`]. À ne pas confondre avec
    /// `ast_valid`, qui parle d'une tentative qui a eu lieu.
    #[serde(default)]
    pub aucun_parseur: bool,
}
