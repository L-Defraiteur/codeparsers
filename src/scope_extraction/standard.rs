//! **Les méthodes de la bibliothèque standard, par langage** : une donnée
//! déclarée, tenue ici et non en dur chez un consommateur.
//!
//! Rust, C++ (et C) et Python lisent `data/std_methods/<langue>.toml`, généré
//! depuis la bibliothèque standard du poste par `data/std_methods/generer.py`
//! (la doc de std de la chaîne épinglée, l'AST de clang sur libstdc++,
//! `dir()` de l'interpréteur) — jamais deviné (15 novembre 2026). Go, C# et
//! TypeScript gardent la liste relevée le 11 octobre 2026 sur les noms les
//! plus « manqués ».
//!
//! Deux usages : un appel sur une locale sans type lu dont le nom est ici ne se
//! relie pas par le nom entre fichiers (`receiver_type_unread`) ; le banc de
//! couverture le compte parmi les **externes**, pas parmi les liens manqués.

use crate::parallel::parser_worker::SupportedLanguage;

/// Les fichiers livrés : (langue, texte).
const DONNEES: &[(&str, &str)] = &[
    ("rust", include_str!("../../data/std_methods/rust.toml")),
    ("cpp", include_str!("../../data/std_methods/cpp.toml")),
    ("python", include_str!("../../data/std_methods/python.toml")),
];

#[derive(serde::Deserialize)]
struct Liste {
    #[allow(dead_code)]
    langue: String,
    #[allow(dead_code)]
    source: String,
    #[allow(dead_code)]
    types: Vec<String>,
    noms: Vec<String>,
}

fn donnees(langue: &str) -> &'static std::collections::HashSet<String> {
    static LUES: std::sync::OnceLock<std::collections::HashMap<&'static str, std::collections::HashSet<String>>> = std::sync::OnceLock::new();
    static VIDE: std::sync::OnceLock<std::collections::HashSet<String>> = std::sync::OnceLock::new();
    LUES.get_or_init(|| {
        DONNEES
            .iter()
            .map(|(l, t)| {
                let liste: Liste = toml::from_str(t).unwrap_or_else(|e| panic!("data/std_methods/{l}.toml livré illisible : {e}"));
                (*l, liste.noms.into_iter().collect())
            })
            .collect()
    })
    .get(langue)
    .unwrap_or_else(|| VIDE.get_or_init(Default::default))
}

const GO: &[&str] = &["Len", "Lock", "Unlock", "RLock", "RUnlock", "Add", "Done", "Wait", "Error", "String", "Close", "Read", "Write"];

const CSHARP: &[&str] = &[
    "Add", "Remove", "Contains", "ContainsKey", "TryGetValue", "Count", "ToString", "Equals", "GetHashCode", "Select", "Where",
    "ToList", "ToArray", "First", "FirstOrDefault", "Any", "Clear", "Dispose",
];

const TS: &[&str] = &[
    "push", "pop", "map", "filter", "reduce", "forEach", "find", "includes", "indexOf", "slice", "splice", "join", "split",
    "get", "set", "has", "delete", "keys", "values", "entries", "toString", "then", "catch",
];

/// **The stop words of a language, as data** (`data/stop_words/<langue>.toml`, generated from the machine
/// by `data/stop_words/generer.py`, 16 November 2026) : an identifier of the list is not a reference.
/// `None` for a language whose list is still in code.
pub fn stop_words_of(language: &SupportedLanguage) -> Option<Vec<String>> {
    const STOP_WORDS: &[(&str, &str)] = &[("rust", include_str!("../../data/stop_words/rust.toml"))];
    let langue = match language {
        SupportedLanguage::Rust => "rust",
        _ => return None,
    };
    let texte = STOP_WORDS.iter().find(|(l, _)| *l == langue)?.1;
    #[derive(serde::Deserialize)]
    struct Liste {
        noms: Vec<String>,
    }
    let liste: Liste = toml::from_str(texte).unwrap_or_else(|e| panic!("data/stop_words/{langue}.toml livré illisible : {e}"));
    Some(liste.noms)
}

/// `nom` est-il une méthode de la bibliothèque standard du langage ?
pub fn is_standard_method_of(language: &SupportedLanguage, nom: &str) -> bool {
    match language {
        SupportedLanguage::Cpp | SupportedLanguage::C => donnees("cpp").contains(nom),
        SupportedLanguage::Rust => donnees("rust").contains(nom),
        SupportedLanguage::Python => donnees("python").contains(nom),
        SupportedLanguage::Go => GO.contains(&nom),
        SupportedLanguage::Csharp => CSHARP.contains(&nom),
        SupportedLanguage::Typescript | SupportedLanguage::Javascript => TS.contains(&nom),
    }
}

/// `nom` est-il une méthode standard du langage de `fichier` ?
pub fn is_standard_method(fichier: &str, nom: &str) -> bool {
    crate::parallel::project_parser::detect_language_from_path(fichier).is_some_and(|l| is_standard_method_of(&l, nom))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_noms_standard_se_lisent_par_le_fichier() {
        assert!(is_standard_method("a/b.cpp", "push_back"));
        assert!(is_standard_method("a/b.rs", "collect"));
        // `push_back` est aussi `VecDeque::push_back` ; `emplace_back` n'est que C++.
        assert!(!is_standard_method("a/b.rs", "emplace_back"));
        assert!(!is_standard_method("a/b.cpp", "getSelSize"));
        assert!(!is_standard_method("a/b.inconnu", "size"));
        // Les cas du ticket du 7 novembre : `Duration::is_zero`, `Vec::split_off`.
        assert!(is_standard_method("a/b.rs", "is_zero") && is_standard_method("a/b.rs", "split_off"));
        assert!(is_standard_method("a/b.py", "startswith"));
    }
}
