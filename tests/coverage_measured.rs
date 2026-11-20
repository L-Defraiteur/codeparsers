//! **La couverture de l'analyse, mesurée** (lot 1 de l'intra-scope) : la catégorie de nœuds précis, par langue.

use codeparsers::coverage::{categories, Category};

fn categorie(path: &str, source: &str, texte: &str, ligne: usize) -> Option<Category> {
    categories(path, source)?.into_iter().find(|(_, t, (l, _), _)| t == texte && *l == ligne).map(|(_, _, _, c)| c)
}

#[test]
fn each_named_node_falls_in_its_category() {
    let cas: Vec<(&str, &str, Vec<(&str, usize, Category)>)> = vec![
        (
            "lib.rs",
            "// un commentaire\nfn g() -> u32 { 1 }\nfn f() -> u32 {\n    let x = g();\n    x + 2\n}\n",
            vec![("f", 3, Category::Declaration), ("g", 4, Category::Reference), ("x", 4, Category::Excluded), ("x", 5, Category::Excluded), ("2", 5, Category::Neutral)],
        ),
        (
            "chemin.rs",
            "mod a { pub fn h() -> u32 { 1 } }\nfn f() -> u32 {\n    a::h()\n}\n",
            vec![("a::h", 3, Category::Structure), ("h", 3, Category::Reference)],
        ),
        (
            "app.py",
            "def g():\n    return 1\n\ndef f(y):\n    x = g()\n    return x + y + len('a')\n",
            vec![("f", 4, Category::Declaration), ("g", 5, Category::Reference), ("y", 6, Category::Excluded), ("len", 6, Category::Excluded)],
        ),
        (
            "app.ts",
            "function g(): number { return 1; }\nexport function f(): number {\n    const x = g();\n    return x + 3;\n}\n",
            vec![("f", 2, Category::Declaration), ("g", 3, Category::Reference), ("x", 4, Category::Excluded), ("3", 4, Category::Neutral)],
        ),
    ];
    let mut faux = Vec::new();
    for (path, source, attendus) in cas {
        for (texte, ligne, attendue) in attendus {
            let eue = categorie(path, source, texte, ligne);
            if eue != Some(attendue) {
                faux.push(format!("{path} : « {texte} » ligne {ligne} attendu {attendue:?}, eu {eue:?}"));
            }
        }
    }
    assert!(faux.is_empty(), "{faux:#?}");
}
