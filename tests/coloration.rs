//! La coloration par les grammaires (`syntax_highlighting::highlight`) : quelques spans attendus par
//! langue, pas l'égalité de toute la liste (conception : page 12 de la session codeparsers, rag3db).

use codeparsers::syntax_highlighting::{highlight, langues, Sorte, Span};

fn sorte_de(spans: &[Span], texte: &str, motif: &str) -> Option<Sorte> {
    let debut = texte.find(motif).unwrap_or_else(|| panic!("« {motif} » absent de l'extrait"));
    let fin = debut + motif.len();
    spans.iter().find(|s| s.debut <= debut && fin <= s.fin).map(|s| s.sorte)
}

fn attendu(langue: &str, texte: &str, cas: &[(&str, Sorte)]) {
    let spans = highlight(texte, langue).unwrap_or_else(|| panic!("{langue} : aucune coloration"));
    for w in spans.windows(2) {
        assert!(w[0].fin <= w[1].debut, "{langue} : spans non triés ou recouvrants {w:?}");
    }
    for (motif, sorte) in cas {
        assert_eq!(sorte_de(&spans, texte, motif), Some(*sorte), "{langue} : « {motif} »");
    }
}

#[test]
fn chaque_langue_annoncee_compile_sa_requete() {
    for langue in langues() {
        assert!(highlight("", langue).is_some(), "{langue}");
    }
    assert!(highlight("x", "cobol").is_none());
}

#[test]
fn rust() {
    let t = "// lire\nfn vue_portee(p: &Path) -> Option<u32> {\n    let n = compter(\"a\");\n    Some(42)\n}\n";
    attendu("rust", t, &[
        ("// lire", Sorte::Comment), ("fn", Sorte::Keyword), ("vue_portee", Sorte::Function),
        ("Path", Sorte::Type), ("compter", Sorte::Function), ("\"a\"", Sorte::String),
        ("42", Sorte::Constant),
    ]);
}

#[test]
fn cpp_lit_la_requete_de_c_puis_la_sienne() {
    let t = "// note\ntemplate <typename T>\nint taille(const T& v) {\n    return compter(v, \"x\");\n}\n";
    attendu("c++", t, &[
        ("// note", Sorte::Comment), ("template", Sorte::Keyword), ("return", Sorte::Keyword),
        ("int", Sorte::Type), ("taille", Sorte::Function), ("\"x\"", Sorte::String),
    ]);
}

#[test]
fn typescript_lit_la_requete_de_javascript_puis_la_sienne() {
    let t = "// note\nconst n: number = compter(\"a\");\ninterface Vue { nom: string }\n";
    attendu("ts", t, &[
        ("// note", Sorte::Comment), ("const", Sorte::Keyword), ("interface", Sorte::Keyword),
        ("number", Sorte::Type), ("Vue", Sorte::Type), ("compter", Sorte::Function),
        ("\"a\"", Sorte::String),
    ]);
}

#[test]
fn python() {
    let t = "# note\ndef vue(p):\n    return compter(\"a\", 3)\n";
    attendu("py", t, &[
        ("# note", Sorte::Comment), ("def", Sorte::Keyword), ("return", Sorte::Keyword),
        ("vue", Sorte::Function), ("compter", Sorte::Function), ("\"a\"", Sorte::String),
        ("3", Sorte::Number),
    ]);
}

#[test]
fn un_texte_qui_ne_se_lit_pas_se_colore_la_ou_l_arbre_tient() {
    let t = "fn ok() { let s = \"a\"; }\nfn casse( {{{\n";
    attendu("rust", t, &[("fn", Sorte::Keyword), ("\"a\"", Sorte::String)]);
}

#[test]
fn les_decalages_sont_des_octets_du_texte() {
    let t = "// é\nlet x = \"ü\";\n";
    let spans = highlight(t, "rust").unwrap();
    let chaine = spans.iter().find(|s| s.sorte == Sorte::String).expect("une chaîne");
    assert_eq!(&t[chaine.debut..chaine.fin], "\"ü\"");
}
