//! **Les règles d'import par langue, en données** (14 novembre 2026, conception
//! `docs/codeparsers/14-11-2026/02-du-nom-importe-au-paquet.md`, rag3db). Un fichier TOML par langue
//! (`data/imports/<langue>.toml`) dit la bibliothèque standard, la forme locale d'un import et la règle
//! du nom importé au nom de paquet. Les fichiers livrés sont **compilés comme défaut** ; un fichier du
//! même nom dans le dossier `import_rules_dir` **remplace la langue entière** (pas de fusion champ à
//! champ), et l'origine des règles de chaque langue se dit (`Regles::origines`).
//!
//! Les règles **nommées** (`premier_segment_sans_point`, `plus_long_prefixe_declare`, `aucune`) sont
//! une liste fermée ici : une règle neuve est du code, une liste ou un seuil est de la donnée.

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex, OnceLock};

/// Les fichiers livrés, compilés comme défaut : (langue, texte).
const DEFAUTS: &[(&str, &str)] = &[("python", include_str!("../../data/imports/python.toml"))];

/// Une règle nommée : du code, choisi par les données.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RegleNommee {
    /// Go : un chemin dont le premier élément n'a pas de point est la bibliothèque standard.
    PremierSegmentSansPoint,
    /// Go : le module déclaré par `go.mod` est le plus long préfixe déclaré du chemin importé.
    PlusLongPrefixeDeclare,
    /// Le nom ne dit rien du paquet (Java, Ruby, C++) : seule la table déclarée du dépôt tranche.
    Aucune,
}

/// Une variante de normalisation : deux noms qui ne diffèrent que par elle se comparent égaux.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Variante {
    /// `serde-json` et `serde_json`.
    TiretSouligne,
    /// `PyYAML` et `pyyaml`.
    Casse,
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Builtin {
    /// D'où vient la liste (une version, une commande), dite telle quelle.
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub noms: Vec<String>,
    #[serde(default)]
    pub prefixes: Vec<String>,
    #[serde(default)]
    pub regle: Option<RegleNommee>,
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Local {
    #[serde(default)]
    pub prefixes: Vec<String>,
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NomDePaquet {
    /// Le séparateur des segments du nom importé (`.`, `/`, `::`).
    #[serde(default)]
    pub separateur: Option<String>,
    /// Combien de segments forment le nom de paquet (1 : `requests.adapters` → `requests`).
    #[serde(default)]
    pub garder: Option<usize>,
    /// Plus de segments quand le premier commence par un préfixe (`{ "@" = 2 }` : `@scope/pkg/sub` →
    /// `@scope/pkg`).
    #[serde(default)]
    pub garder_si_le_premier_commence_par: BTreeMap<String, usize>,
    #[serde(default)]
    pub variantes: Vec<Variante>,
    #[serde(default)]
    pub regle: Option<RegleNommee>,
}

/// Les règles d'une langue.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegleDeLangue {
    pub langue: String,
    pub extensions: Vec<String>,
    #[serde(default)]
    pub builtin: Builtin,
    #[serde(default)]
    pub local: Local,
    #[serde(default)]
    pub nom_de_paquet: NomDePaquet,
}

impl RegleDeLangue {
    fn tete(&self, source: &str) -> Vec<String> {
        let s = source.trim();
        let sep = self.nom_de_paquet.separateur.as_deref().unwrap_or("");
        if sep.is_empty() {
            return vec![s.to_string()];
        }
        s.split(sep).map(String::from).collect()
    }

    /// L'import est-il de la bibliothèque standard ?
    pub fn est_builtin(&self, source: &str) -> bool {
        let s = source.trim();
        if self.builtin.prefixes.iter().any(|p| s.starts_with(p.as_str())) {
            return true;
        }
        match self.builtin.regle {
            Some(RegleNommee::PremierSegmentSansPoint) => {
                return s.split('/').next().is_some_and(|r| !r.is_empty() && !r.contains('.'));
            }
            Some(_) => return false,
            None => {}
        }
        let premier = self.tete(s).into_iter().next().unwrap_or_default();
        self.builtin.noms.iter().any(|n| n == &premier || n == s)
    }

    /// L'import a-t-il la forme locale de la langue (relatif) ?
    pub fn est_local(&self, source: &str) -> bool {
        let s = source.trim();
        self.local.prefixes.iter().any(|p| s.starts_with(p.as_str()))
    }

    /// Les noms de paquet candidats, normalisés, dans l'ordre (vide : le nom ne dit rien).
    pub fn candidats(&self, source: &str) -> Vec<String> {
        if self.nom_de_paquet.regle == Some(RegleNommee::Aucune) {
            return Vec::new();
        }
        let segments = self.tete(source);
        let Some(premier) = segments.first() else { return Vec::new() };
        let mut garder = self.nom_de_paquet.garder.unwrap_or(segments.len()).max(1);
        for (prefixe, n) in &self.nom_de_paquet.garder_si_le_premier_commence_par {
            if premier.starts_with(prefixe.as_str()) {
                garder = *n;
            }
        }
        let sep = self.nom_de_paquet.separateur.as_deref().unwrap_or("");
        let nom = segments.iter().take(garder).cloned().collect::<Vec<_>>().join(sep);
        vec![self.normaliser(&nom)]
    }

    /// Un nom normalisé par les variantes de la langue (le même pour un nom importé et un nom de manifeste).
    pub fn normaliser(&self, nom: &str) -> String {
        let mut n = nom.trim().to_string();
        for v in &self.nom_de_paquet.variantes {
            n = match v {
                Variante::TiretSouligne => n.replace('-', "_"),
                Variante::Casse => n.to_lowercase(),
            };
        }
        n
    }
}

/// Les règles de toutes les langues qui en ont, et d'où elles viennent.
#[derive(Debug, Default)]
pub struct Regles {
    par_extension: HashMap<String, Arc<RegleDeLangue>>,
    /// Par langue : `défaut` ou le chemin du fichier qui la remplace (ou pourquoi il a été refusé).
    pub origines: BTreeMap<String, String>,
}

impl Regles {
    /// Les règles de la langue d'un fichier (par son extension), si elle en a.
    pub fn de_fichier(&self, fichier: &str) -> Option<&RegleDeLangue> {
        let ext = fichier.rsplit('.').next().unwrap_or("");
        self.par_extension.get(ext).map(|r| r.as_ref())
    }

    /// Les défauts compilés, remplacés langue par langue par les fichiers de `dossier`.
    pub fn charger(dossier: Option<&str>) -> Regles {
        let mut r = Regles::default();
        for (langue, texte) in DEFAUTS {
            let mut origine = "défaut".to_string();
            let mut regle: RegleDeLangue = toml::from_str(texte).unwrap_or_else(|e| panic!("data/imports/{langue}.toml livré illisible : {e}"));
            if let Some(d) = dossier {
                let chemin = std::path::Path::new(d).join(format!("{langue}.toml"));
                if let Ok(t) = std::fs::read_to_string(&chemin) {
                    match toml::from_str::<RegleDeLangue>(&t) {
                        Ok(remplacee) => {
                            regle = remplacee;
                            origine = chemin.display().to_string();
                        }
                        Err(e) => origine = format!("défaut ({} refusé : {e})", chemin.display()),
                    }
                }
            }
            let regle = Arc::new(regle);
            for ext in &regle.extensions {
                r.par_extension.insert(ext.clone(), regle.clone());
            }
            r.origines.insert(langue.to_string(), origine);
        }
        r
    }

    /// Les règles chargées une fois par dossier (une analyse en lots ne relit pas les fichiers).
    pub fn partagees(dossier: Option<&str>) -> Arc<Regles> {
        static CACHE: OnceLock<Mutex<HashMap<Option<String>, Arc<Regles>>>> = OnceLock::new();
        let cle = dossier.map(String::from);
        let mut c = CACHE.get_or_init(Default::default).lock().unwrap_or_else(|e| e.into_inner());
        c.entry(cle).or_insert_with(|| Arc::new(Regles::charger(dossier))).clone()
    }
}
