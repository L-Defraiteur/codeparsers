//! **Les méthodes de la bibliothèque standard, par langage** : une donnée
//! déclarée, tenue ici et non en dur chez un consommateur.
//!
//! Un appel non résolu à l'un de ces noms, sur un receveur dont le type ne se
//! lit pas, est presque toujours un appel à la bibliothèque standard
//! (`v.size()`, `xs.collect()`), même quand le projet définit un homonyme : il
//! compte parmi les **externes**, pas parmi les liens manqués (le compteur
//! d'absents du banc de couverture, et la même règle au rendez-vous de
//! rag3weaver). La liste vient des noms les plus « manqués » relevés le
//! 11 octobre 2026 sur rag3db/src et rag3weaver.

use crate::parallel::parser_worker::SupportedLanguage;

const CPP: &[&str] = &[
    "size", "push_back", "emplace_back", "pop_back", "get", "empty", "insert", "emplace", "erase", "find", "contains", "count",
    "begin", "end", "cbegin", "cend", "rbegin", "rend", "front", "back", "at", "clear", "reserve", "resize", "data", "c_str",
    "length", "substr", "append", "reset", "release", "swap", "copy", "toString", "str", "first", "second", "lock", "unlock",
    "load", "store", "wait", "notify_one", "notify_all", "join", "value", "has_value", "value_or",
];

const RUST: &[&str] = &[
    "collect", "clone", "to_string", "to_owned", "as_str", "as_ref", "as_mut", "len", "is_empty", "join", "push", "pop", "get",
    "get_mut", "insert", "remove", "contains", "contains_key", "iter", "iter_mut", "into_iter", "map", "filter", "unwrap",
    "unwrap_or", "unwrap_or_default", "unwrap_or_else", "expect", "ok", "ok_or", "ok_or_else", "map_err", "and_then", "lock",
    "read", "write", "entry", "or_default", "or_insert", "or_insert_with", "extend", "first", "last", "split", "trim",
    "starts_with", "ends_with", "parse", "into", "from", "sort", "sort_by", "dedup", "retain", "drain", "clear", "send", "recv",
    "keys", "values", "chars", "bytes", "lines", "find", "any", "all", "count", "sum", "min", "max", "take", "skip", "next",
];

const PYTHON: &[&str] = &[
    "append", "extend", "insert", "pop", "remove", "get", "items", "keys", "values", "update", "setdefault", "join", "split",
    "strip", "startswith", "endswith", "format", "lower", "upper", "replace", "encode", "decode", "read", "write", "close",
];

const GO: &[&str] = &["Len", "Lock", "Unlock", "RLock", "RUnlock", "Add", "Done", "Wait", "Error", "String", "Close", "Read", "Write"];

const CSHARP: &[&str] = &[
    "Add", "Remove", "Contains", "ContainsKey", "TryGetValue", "Count", "ToString", "Equals", "GetHashCode", "Select", "Where",
    "ToList", "ToArray", "First", "FirstOrDefault", "Any", "Clear", "Dispose",
];

const TS: &[&str] = &[
    "push", "pop", "map", "filter", "reduce", "forEach", "find", "includes", "indexOf", "slice", "splice", "join", "split",
    "get", "set", "has", "delete", "keys", "values", "entries", "toString", "then", "catch",
];

/// Les noms de méthodes standard du langage.
pub fn standard_methods(language: &SupportedLanguage) -> &'static [&'static str] {
    match language {
        SupportedLanguage::Cpp | SupportedLanguage::C => CPP,
        SupportedLanguage::Rust => RUST,
        SupportedLanguage::Python => PYTHON,
        SupportedLanguage::Go => GO,
        SupportedLanguage::Csharp => CSHARP,
        SupportedLanguage::Typescript | SupportedLanguage::Javascript => TS,
    }
}

/// `nom` est-il une méthode standard du langage de `fichier` ?
pub fn is_standard_method(fichier: &str, nom: &str) -> bool {
    crate::parallel::project_parser::detect_language_from_path(fichier).is_some_and(|l| standard_methods(&l).contains(&nom))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_noms_standard_se_lisent_par_le_fichier() {
        assert!(is_standard_method("a/b.cpp", "push_back"));
        assert!(is_standard_method("a/b.rs", "collect"));
        assert!(!is_standard_method("a/b.rs", "push_back"));
        assert!(!is_standard_method("a/b.cpp", "getSelSize"));
        assert!(!is_standard_method("a/b.inconnu", "size"));
    }
}
