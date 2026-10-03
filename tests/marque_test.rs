//! **Savoir qu'un scope est un test, et à quel point on en est sûr.**
//!
//! Chaque scope sort avec une marque de test (`ScopeInfo.test`) : son rôle
//! (`Case`, un test ; `Suite`, ce qui en regroupe ; `Support`, du code qui
//! n'existe que pour les tests) et sa certitude — `Certain` quand la syntaxe
//! ou l'outil le dit (`#[test]`, `#[cfg(test)]`, `func TestX(t *testing.T)`
//! dans un `_test.go`, une sous-classe de `unittest.TestCase`, `TEST(…)` de
//! gtest, `describe`/`it`), `Convention` quand seul le nom le dit (pytest :
//! `test_*` dans un fichier de test, classes `Test*`).

use codeparsers::parallel::project_parser::{ParseProjectOptions, ProjectParser, ProjectParserOptions};
use codeparsers::scope_extraction::types::{ScopeInfo, TestCertainty, TestRole};
use std::collections::HashMap;

fn scopes(nom: &str, source: &str) -> Vec<ScopeInfo> {
    let chemin = format!("/virtual/{nom}");
    let mut a = ProjectParser::new(ProjectParserOptions { verbose: false }).parse_project(ParseProjectOptions {
        root: "/virtual".to_string(),
        files: vec![chemin.clone()],
        content_map: Some(HashMap::from([(chemin.clone(), source.to_string())])),
        resolve_relationships: Some(false),
        resolver_options: None,
    });
    a.files.remove(&chemin).expect("le fichier analysé").scopes
}

fn marque(scopes: &[ScopeInfo], nom: &str) -> Option<(TestRole, TestCertainty, Option<String>)> {
    let s = scopes.iter().find(|s| s.name == nom).unwrap_or_else(|| {
        panic!("pas de scope {nom} ; scopes : {:?}", scopes.iter().map(|s| &s.name).collect::<Vec<_>>())
    });
    s.test.as_ref().map(|t| (t.role.clone(), t.certainty.clone(), t.name.clone()))
}

fn est(scopes: &[ScopeInfo], nom: &str, role: TestRole, certitude: TestCertainty) {
    let m = marque(scopes, nom);
    assert!(
        m.as_ref().is_some_and(|(r, c, _)| *r == role && *c == certitude),
        "{nom} : attendu {role:?}/{certitude:?}, trouvé {m:?}"
    );
}

fn pas_un_test(scopes: &[ScopeInfo], nom: &str) {
    assert_eq!(marque(scopes, nom), None, "{nom} n'est pas un test");
}

// ---------------------------------------------------------------------------
// Rust
// ---------------------------------------------------------------------------

const RUST: &str = r#"pub fn add(a: u32, b: u32) -> u32 { a + b }

pub fn test_looks_like_a_test() {}

mod outils {
    pub fn plain() -> u32 { 1 }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn helper() -> u32 { 2 }

    #[test]
    fn adds() {
        assert_eq!(add(1, helper()), 3);
    }

    #[tokio::test]
    async fn adds_async() {}
}
"#;

#[test]
fn rust_la_syntaxe_dit_ce_qui_est_un_test() {
    let s = scopes("lib.rs", RUST);
    est(&s, "adds", TestRole::Case, TestCertainty::Certain);
    est(&s, "adds_async", TestRole::Case, TestCertainty::Certain);
    est(&s, "tests", TestRole::Suite, TestCertainty::Certain);
    est(&s, "helper", TestRole::Support, TestCertainty::Certain);
    pas_un_test(&s, "add");
    pas_un_test(&s, "plain");
    pas_un_test(&s, "test_looks_like_a_test");
}

// ---------------------------------------------------------------------------
// Python
// ---------------------------------------------------------------------------

const PY: &str = r#"import unittest
import pytest

@pytest.fixture
def base():
    return 1

def test_add(base):
    assert base == 1

class TestCalc:
    def test_sub(self):
        pass

class CalcCase(unittest.TestCase):
    def test_mul(self):
        pass
    def helper(self):
        pass
"#;

#[test]
fn python_pytest_est_une_convention_unittest_une_syntaxe() {
    let s = scopes("test_calc.py", PY);
    est(&s, "base", TestRole::Support, TestCertainty::Certain);
    est(&s, "test_add", TestRole::Case, TestCertainty::Convention);
    est(&s, "TestCalc", TestRole::Suite, TestCertainty::Convention);
    est(&s, "test_sub", TestRole::Case, TestCertainty::Convention);
    est(&s, "CalcCase", TestRole::Suite, TestCertainty::Certain);
    est(&s, "test_mul", TestRole::Case, TestCertainty::Certain);
    est(&s, "helper", TestRole::Support, TestCertainty::Certain);
}

#[test]
fn python_hors_d_un_fichier_de_test_le_nom_ne_suffit_pas() {
    let s = scopes("calc.py", "def test_add():\n    pass\n\nclass TestThing:\n    pass\n");
    pas_un_test(&s, "test_add");
    pas_un_test(&s, "TestThing");
}

// ---------------------------------------------------------------------------
// C++ (gtest)
// ---------------------------------------------------------------------------

#[test]
fn cpp_les_macros_gtest_sont_des_tests_et_disent_leur_nom() {
    let src = "#include <gtest/gtest.h>\nTEST(CalcTest, Adds) {\n  EXPECT_EQ(1, 1);\n}\nTEST_F(CalcFixture, Subs) {\n  EXPECT_EQ(1, 1);\n}\nint helper() { return 1; }\n";
    let s = scopes("calc_test.cpp", src);
    let tests: Vec<_> = s.iter().filter_map(|x| x.test.as_ref().map(|t| (t.role.clone(), t.certainty.clone(), t.name.clone()))).collect();
    assert!(tests.contains(&(TestRole::Case, TestCertainty::Certain, Some("CalcTest.Adds".to_string()))), "{tests:?}");
    assert!(tests.contains(&(TestRole::Case, TestCertainty::Certain, Some("CalcFixture.Subs".to_string()))), "{tests:?}");
    pas_un_test(&s, "helper");
}

// ---------------------------------------------------------------------------
// Go
// ---------------------------------------------------------------------------

#[test]
fn go_la_signature_et_le_fichier_disent_le_test() {
    let src = "package calc\n\nimport \"testing\"\n\nfunc TestAdd(t *testing.T) {\n\tt.Log(1)\n}\n\nfunc helper() int { return 1 }\n";
    let s = scopes("calc_test.go", src);
    est(&s, "TestAdd", TestRole::Case, TestCertainty::Certain);
    est(&s, "helper", TestRole::Support, TestCertainty::Certain);
    let ailleurs = scopes("calc.go", "package calc\n\nfunc TestLike() {}\n");
    pas_un_test(&ailleurs, "TestLike");
}

// ---------------------------------------------------------------------------
// TypeScript / JavaScript
// ---------------------------------------------------------------------------

#[test]
fn ts_describe_et_it_deviennent_des_scopes_de_test() {
    let src = "import { add } from \"./calc\";\ndescribe(\"calc\", () => {\n  it(\"adds\", () => {\n    expect(add(1, 2)).toBe(3);\n  });\n  test(\"subs\", () => {});\n});\n";
    let s = scopes("calc.test.ts", src);
    let tests: Vec<_> = s.iter().filter_map(|x| x.test.as_ref().map(|t| (t.role.clone(), t.name.clone()))).collect();
    assert!(tests.contains(&(TestRole::Suite, Some("calc".to_string()))), "{tests:?}");
    assert!(tests.contains(&(TestRole::Case, Some("calc > adds".to_string()))), "{tests:?}");
    assert!(tests.contains(&(TestRole::Case, Some("calc > subs".to_string()))), "{tests:?}");
    let adds = s.iter().find(|x| x.name == "adds").expect("le scope du test");
    assert_eq!((adds.scope_start_line, adds.scope_end_line, adds.parent.as_deref()), (3, 5, Some("calc")));
    assert!(adds.identifier_references.iter().any(|r| r.identifier == "add"), "le test référence add");
}

#[test]
fn ts_un_appel_ordinaire_ne_devient_pas_un_scope() {
    let s = scopes("app.ts", "function run(f: () => void) { f(); }\nrun(() => {});\nlog(\"x\", () => {});\n");
    assert!(s.iter().all(|x| x.test.is_none()), "{:?}", s.iter().map(|x| &x.name).collect::<Vec<_>>());
    assert!(!s.iter().any(|x| x.name == "x"));
}
