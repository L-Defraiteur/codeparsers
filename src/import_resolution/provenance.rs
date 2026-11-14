//! **La provenance d'un nom importé** (14 novembre 2026, décision de Lucie relayée par l'orchestration) :
//! `builtin` (la bibliothèque standard du langage), `project` (un module du dépôt), `dependency` (une
//! dépendance d'un gestionnaire de paquets) ou `inconnue` — jamais une devinette. codeparsers tranche
//! `builtin` par des listes fixes par langage et `project` par la forme de l'import ; `dependency`
//! demande les manifestes du dépôt (Cargo.lock, package.json…), que lit le consommateur (rag3weaver) :
//! ici, ce qui n'est ni l'un ni l'autre reste `inconnue`.

use crate::import_resolution::c_import_resolver::{CPP_STDLIB_HEADERS, C_STDLIB_HEADERS};
use crate::import_resolution::c_sharp_import_resolver::DOTNET_BCL_NAMESPACES;
use crate::import_resolution::rust_import_resolver::RUST_STD_CRATES;
pub use crate::import_resolution::regles::Regles;

/// La provenance d'un nom importé.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Provenance {
    #[serde(rename = "builtin")]
    Builtin,
    #[serde(rename = "dependency")]
    Dependency,
    #[serde(rename = "project")]
    Project,
    #[serde(rename = "inconnue")]
    Unknown,
}

impl Provenance {
    pub fn as_str(self) -> &'static str {
        match self {
            Provenance::Builtin => "builtin",
            Provenance::Dependency => "dependency",
            Provenance::Project => "project",
            Provenance::Unknown => "inconnue",
        }
    }
}

/// L'import `source` d'un fichier `fichier` (son extension dit le langage) vient-il de la bibliothèque
/// standard du langage ?
pub fn est_builtin(fichier: &str, source: &str) -> bool {
    est_builtin_selon(&Regles::partagees(None), fichier, source)
}

/// [`est_builtin`] selon des règles chargées (`import_rules_dir`) : une langue qui a son fichier de
/// règles passe par lui, les autres par le code ci-dessous.
pub fn est_builtin_selon(regles: &Regles, fichier: &str, source: &str) -> bool {
    if let Some(r) = regles.de_fichier(fichier) {
        return r.est_builtin(source);
    }
    let ext = fichier.rsplit('.').next().unwrap_or("");
    let s = source.trim();
    match ext {
        "rs" => s.split("::").next().is_some_and(|r| RUST_STD_CRATES.contains(&r)),
        "c" | "h" => C_STDLIB_HEADERS.contains(&s),
        "cpp" | "cc" | "cxx" | "hpp" | "hh" | "hxx" => CPP_STDLIB_HEADERS.contains(&s) || C_STDLIB_HEADERS.contains(&s),
        // Go : la règle de la chaîne d'outils — un chemin dont le premier élément n'a pas de point
        // est la bibliothèque standard (`net/http`) ; un module tiers commence par un domaine.
        "go" => s.split('/').next().is_some_and(|r| !r.is_empty() && !r.contains('.')),
        "cs" => DOTNET_BCL_NAMESPACES.iter().any(|n| s == *n || s.starts_with(&format!("{n}."))),
        _ => false,
    }
}


/// Le nom de paquet qu'un import nomme, comparable à un nom de manifeste : la crate (`serde` de
/// `serde::Serialize`), le paquet npm (`lodash` de `lodash/fp`, `@scope/pkg` de `@scope/pkg/sub`), le
/// module racine Python (`requests` de `requests.adapters`), le premier dossier d'un en-tête C/C++
/// (`fmt` de `fmt/format.h`), le chemin de module Go entier.
pub fn nom_de_paquet(fichier: &str, source: &str) -> String {
    nom_de_paquet_selon(&Regles::partagees(None), fichier, source)
}

/// [`nom_de_paquet`] selon des règles chargées ; une règle `aucune` rend un nom vide (rien à comparer).
pub fn nom_de_paquet_selon(regles: &Regles, fichier: &str, source: &str) -> String {
    if let Some(r) = regles.de_fichier(fichier) {
        return r.candidats(source).into_iter().next().unwrap_or_default();
    }
    let ext = fichier.rsplit('.').next().unwrap_or("");
    let s = source.trim();
    let tete = match ext {
        "rs" => s.split("::").next().unwrap_or(s).to_string(),
        "c" | "h" | "cpp" | "cc" | "cxx" | "hpp" | "hh" | "hxx" => s.split('/').next().unwrap_or(s).trim_end_matches(".h").to_string(),
        _ => s.to_string(),
    };
    normaliser(&tete)
}

/// Un nom de paquet comparable : minuscules, `-` et `_` confondus (`serde-json` et `serde_json`,
/// `PyYAML` et `pyyaml`).
pub fn normaliser(nom: &str) -> String {
    nom.trim().to_lowercase().replace('-', "_")
}

/// La provenance d'un import, avec ce que les manifestes du dépôt disent (`dependances`,
/// `paquets_du_projet`, des noms déjà normalisés) : `project` pour un import local ou un paquet du
/// dépôt, `builtin` pour la bibliothèque standard, `dependency` pour une dépendance déclarée,
/// sinon `inconnue`.
pub fn provenance_d_un_import(
    regles: &Regles,
    fichier: &str,
    source: &str,
    est_local: bool,
    dependances: &std::collections::HashSet<String>,
    paquets_du_projet: &std::collections::HashSet<String>,
) -> Provenance {
    if est_local || regles.de_fichier(fichier).is_some_and(|r| r.est_local(source)) {
        return Provenance::Project;
    }
    if est_builtin_selon(regles, fichier, source) {
        return Provenance::Builtin;
    }
    let nom = nom_de_paquet_selon(regles, fichier, source);
    if nom.is_empty() {
        Provenance::Unknown
    } else if paquets_du_projet.contains(&nom) {
        Provenance::Project
    } else if dependances.contains(&nom) {
        Provenance::Dependency
    } else {
        Provenance::Unknown
    }
}
