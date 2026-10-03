//! **Marquer les scopes de test.**
//!
//! Une passe après l'extraction, par langage, sur ce que chaque scope porte
//! déjà (attributs au-dessus de sa déclaration, décorateurs, clauses
//! d'héritage, signature, première ligne) et sur le nom du fichier. Elle ne
//! crée ni ne retire aucun scope.
//!
//! Ce qui est sûr et ce qui est une convention, par langage :
//!
//! | Langage | Sûr (`Certain`) | Convention |
//! |---|---|---|
//! | Rust | `#[test]`, `#[tokio::test]` et les attributs `…::test`, `#[rstest]`, `#[test_case]` ; un `mod` sous `#[cfg(test)]` et tout ce qu'il contient ; un fichier de `tests/` | — |
//! | Python | `@pytest.fixture` ; une sous-classe de `…TestCase` et ses méthodes `test*` | `test_*` et `Test*` dans un fichier `test_*.py` / `*_test.py` ; `conftest.py` |
//! | C++ | `TEST`, `TEST_F`, `TEST_P`, `TYPED_TEST`, `TYPED_TEST_P` (gtest), `TEST_CASE` (Catch2) | — |
//! | Go | `TestX(t *testing.T)`, `BenchmarkX(b *testing.B)`, `FuzzX(f *testing.F)`, `ExampleX()` dans un `_test.go`, et le reste de ce fichier | — |

use super::types::{ScopeInfo, ScopeInfoType, TestCertainty, TestMark, TestRole};

fn mark(role: TestRole, certainty: TestCertainty, marker: &str, name: Option<String>) -> Option<TestMark> {
    Some(TestMark { role, certainty, marker: marker.to_string(), name })
}

/// Pose `ScopeInfo.test` sur les scopes d'un fichier.
pub fn mark_tests(scopes: &mut [ScopeInfo], content: &str, file_path: &str) {
    let lines: Vec<&str> = content.split('\n').collect();
    let file_name = file_path.rsplit(['/', '\\']).next().unwrap_or(file_path);
    if file_name.ends_with(".rs") {
        mark_rust(scopes, &lines, file_path);
    } else if file_name.ends_with(".py") {
        mark_python(scopes, file_name);
    } else if file_name.ends_with("_test.go") {
        mark_go(scopes);
    } else if [".cpp", ".cc", ".cxx", ".hpp", ".h", ".hh"].iter().any(|e| file_name.ends_with(e)) {
        mark_cpp(scopes, &lines);
    }
}

/// Les attributs écrits au-dessus de la ligne `ligne` (1 = première),
/// commentaires de doc et lignes vides sautés.
fn attributes_above(lines: &[&str], ligne: usize) -> Vec<String> {
    let mut attrs = Vec::new();
    let mut i = ligne.saturating_sub(1);
    while i > 0 {
        let l = lines.get(i - 1).map(|l| l.trim()).unwrap_or("");
        if l.starts_with("#[") {
            attrs.push(l.to_string());
        } else if !(l.starts_with("///") || l.starts_with("//!") || l.is_empty()) {
            break;
        }
        i -= 1;
    }
    attrs
}

fn rust_test_attribute(attr: &str) -> bool {
    let corps = attr.trim_start_matches("#[");
    let chemin: String = corps.chars().take_while(|c| c.is_alphanumeric() || *c == '_' || *c == ':').collect();
    chemin == "test" || chemin.ends_with("::test") || chemin == "rstest" || chemin == "test_case" || chemin == "wasm_bindgen_test"
}

fn mark_rust(scopes: &mut [ScopeInfo], lines: &[&str], file_path: &str) {
    // Un fichier sous `tests/` est une cible de test pour cargo.
    let cible_de_test = file_path.split(['/', '\\']).any(|s| s == "tests");
    let mut suites: Vec<(usize, usize)> = Vec::new();
    for s in scopes.iter_mut() {
        let attrs = attributes_above(lines, s.scope_start_line);
        if matches!(s.r#type, ScopeInfoType::Function | ScopeInfoType::Method) {
            if let Some(a) = attrs.iter().find(|a| rust_test_attribute(a)) {
                s.test = mark(TestRole::Case, TestCertainty::Certain, a, None);
            }
        } else if matches!(s.r#type, ScopeInfoType::Namespace | ScopeInfoType::Module)
            && attrs.iter().any(|a| a.replace(' ', "").starts_with("#[cfg(test)]"))
        {
            s.test = mark(TestRole::Suite, TestCertainty::Certain, "#[cfg(test)]", None);
            suites.push((s.scope_start_line, s.scope_end_line));
        }
    }
    for s in scopes.iter_mut().filter(|s| s.test.is_none()) {
        let dans_une_suite = suites.iter().any(|(d, f)| *d < s.scope_start_line && s.scope_end_line <= *f);
        if dans_une_suite {
            s.test = mark(TestRole::Support, TestCertainty::Certain, "#[cfg(test)]", None);
        } else if cible_de_test && !s.name.starts_with("file_scope") {
            s.test = mark(TestRole::Support, TestCertainty::Certain, "tests/", None);
        }
    }
}

fn mark_python(scopes: &mut [ScopeInfo], file_name: &str) {
    let fichier_de_test = (file_name.starts_with("test_") || file_name.ends_with("_test.py")) && file_name.ends_with(".py");
    let conftest = file_name == "conftest.py";
    // Les classes de tests du fichier : nom → (certitude, marqueur).
    let mut classes: std::collections::HashMap<String, (TestCertainty, String)> = std::collections::HashMap::new();
    for s in scopes.iter_mut().filter(|s| s.r#type == ScopeInfoType::Class) {
        let base_testcase = s.heritage_clauses.iter().flatten().flat_map(|h| h.types.iter()).find(|t| t.ends_with("TestCase"));
        if let Some(b) = base_testcase {
            s.test = mark(TestRole::Suite, TestCertainty::Certain, b, None);
            classes.insert(s.name.clone(), (TestCertainty::Certain, b.clone()));
        } else if fichier_de_test && s.name.starts_with("Test") {
            s.test = mark(TestRole::Suite, TestCertainty::Convention, "Test*", None);
            classes.insert(s.name.clone(), (TestCertainty::Convention, "Test*".to_string()));
        }
    }
    for s in scopes.iter_mut().filter(|s| matches!(s.r#type, ScopeInfoType::Function | ScopeInfoType::Method)) {
        let fixture = s.decorators.iter().flatten().find(|d| d.starts_with("@pytest.fixture"));
        if let Some(d) = fixture {
            s.test = mark(TestRole::Support, TestCertainty::Certain, d, None);
            continue;
        }
        let classe = s.parent.as_ref().and_then(|p| classes.get(p)).cloned();
        if let Some((certitude, marqueur)) = classe {
            let role = if s.name.starts_with("test") { TestRole::Case } else { TestRole::Support };
            s.test = mark(role, certitude, &marqueur, None);
        } else if s.parent.is_none() && fichier_de_test && s.name.starts_with("test") {
            s.test = mark(TestRole::Case, TestCertainty::Convention, "test_*", None);
        } else if conftest || fichier_de_test {
            s.test = mark(TestRole::Support, TestCertainty::Convention, if conftest { "conftest.py" } else { "fichier de test" }, None);
        }
    }
}

fn mark_go(scopes: &mut [ScopeInfo]) {
    for s in scopes.iter_mut().filter(|s| !s.name.starts_with("file_scope")) {
        let sig = s.signature.replace(' ', "");
        let lanceur = [("Test", "*testing.T"), ("Benchmark", "*testing.B"), ("Fuzz", "*testing.F")]
            .into_iter()
            .find(|(prefixe, param)| s.name.starts_with(prefixe) && sig.contains(&param.replace(' ', "")));
        s.test = if let Some((_, param)) = lanceur {
            mark(TestRole::Case, TestCertainty::Certain, param, None)
        } else if s.name.starts_with("Example") && sig.contains(&format!("{}()", s.name)) {
            mark(TestRole::Case, TestCertainty::Certain, "Example", None)
        } else {
            mark(TestRole::Support, TestCertainty::Certain, "_test.go", None)
        };
    }
}

const MACROS_GTEST: [&str; 5] = ["TEST", "TEST_F", "TEST_P", "TYPED_TEST", "TYPED_TEST_P"];

fn mark_cpp(scopes: &mut [ScopeInfo], lines: &[&str]) {
    for s in scopes.iter_mut().filter(|s| s.r#type == ScopeInfoType::Function) {
        let premiere = lines.get(s.scope_start_line.saturating_sub(1)).map(|l| l.trim()).unwrap_or("");
        let Some(args) = premiere.strip_prefix(s.name.as_str()).map(str::trim_start).and_then(|r| r.strip_prefix('(')) else {
            continue;
        };
        let args = args.split(')').next().unwrap_or("");
        if MACROS_GTEST.contains(&s.name.as_str()) {
            let parts: Vec<&str> = args.split(',').map(str::trim).collect();
            if parts.len() == 2 && parts.iter().all(|p| !p.is_empty()) {
                s.test = mark(TestRole::Case, TestCertainty::Certain, &s.name, Some(format!("{}.{}", parts[0], parts[1])));
            }
        } else if s.name == "TEST_CASE" {
            let nom = args.split(',').next().unwrap_or("").trim().trim_matches('"');
            s.test = mark(TestRole::Case, TestCertainty::Certain, "TEST_CASE", (!nom.is_empty()).then(|| nom.to_string()));
        }
    }
}
