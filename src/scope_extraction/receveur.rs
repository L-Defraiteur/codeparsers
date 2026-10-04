//! **Le type d'un receveur Rust, lu à travers une chaîne d'appels de la
//! bibliothèque standard.**
//!
//! `let cat = catalog.lock().unwrap(); cat.ingest_entities(…)` avec
//! `catalog: Arc<Mutex<Catalog>>` : `cat` est un `Catalog`. Le type écrit en
//! entier (pas seulement son nom de base) se suit à travers une petite table
//! de méthodes std dont le retour est certain : `Mutex::lock`,
//! `RwLock::read|write`, `RefCell::borrow`, `unwrap` / `expect` / `?` sur un
//! `Result` ou une `Option`, `clone`, et les itérateurs des collections.
//!
//! **Une règle ne fait que perdre une arête, jamais en inventer** : un
//! receveur typé `Vec` ou `Iterator` vise une méthode de ce type, et le
//! consommateur s'abstient faute de définisseur de ce type dans le projet —
//! c'est le gain (un `x.iter().map(f).collect()` ne se relie plus au
//! `collect` d'un fichier du projet). Une méthode hors de la table arrête la
//! lecture : pas de type, la règle d'avant.

use std::collections::HashMap;

use tree_sitter::Node;

use super::usage::{initializer_type, simple_name, texte};

/// Ce qui se traverse pour appeler une méthode : pointeurs et gardes.
const ENVELOPPES: &[&str] = &["Box", "Arc", "Rc", "MutexGuard", "RwLockReadGuard", "RwLockWriteGuard", "Ref", "RefMut"];

/// Les constructeurs qui enveloppent leur argument : `Arc::new(e)` est un
/// `Arc<type de e>`.
const CONSTRUCTEURS: &[&str] = &["Arc", "Rc", "Box", "Mutex", "RwLock", "RefCell", "Cell"];

/// Le nom d'une enveloppe seule (`Arc`, sans son contenu) : un type qui ne
/// dit rien de ce qu'on appelle à travers lui.
pub fn is_wrapper(t: &str) -> bool {
    CONSTRUCTEURS.contains(&t.trim())
}

/// **Des constructeurs enveloppants en tête** : `Arc::new(Mutex::new(e)).f()`
/// → (`e`, [`wrap:Mutex`, `wrap:Arc`], `.f()`) — le plus intérieur d'abord.
pub fn split_constructors(expr: &str) -> Option<(String, Vec<String>, String)> {
    let expr = expr.trim();
    let tete: String = expr.chars().take_while(|c| c.is_alphanumeric() || *c == '_' || *c == ':').collect();
    let (chemin, fonction) = tete.rsplit_once("::")?;
    let nom = chemin.rsplit("::").next()?;
    if fonction != "new" || !CONSTRUCTEURS.contains(&nom) {
        return None;
    }
    let apres = expr[tete.len()..].trim_start();
    let fin = fin_des_parentheses(apres)?;
    let dedans = apres[1..fin].trim().to_string();
    let reste = apres[fin + 1..].to_string();
    let enveloppe = format!("wrap:{nom}");
    match split_constructors(&dedans) {
        Some((d, mut env, r)) if r.trim().is_empty() => {
            env.push(enveloppe);
            Some((d, env, reste))
        }
        _ => Some((dedans, vec![enveloppe], reste)),
    }
}

/// Les collections std, et leurs méthodes qui rendent un itérateur.
const COLLECTIONS: &[&str] = &["Vec", "VecDeque", "HashMap", "BTreeMap", "HashSet", "BTreeSet", "String", "str", "Option", "Result"];
const VERS_ITERATEUR: &[&str] = &[
    "iter", "iter_mut", "into_iter", "keys", "values", "values_mut", "into_keys", "into_values", "drain", "chars", "bytes", "lines",
    "split", "split_whitespace", "windows", "chunks",
];

/// Les adaptateurs d'un itérateur, qui rendent un itérateur.
const ADAPTATEURS: &[&str] = &[
    "map", "filter", "filter_map", "flat_map", "flatten", "enumerate", "zip", "chain", "take", "skip", "rev", "cloned", "copied",
    "peekable", "inspect", "take_while", "skip_while", "step_by", "map_while", "scan", "fuse",
];

/// Un type écrit, découpé : son nom (dernier segment du chemin) et ses
/// arguments génériques de premier niveau. `&mut std::sync::Arc<Mutex<T>>`
/// → (`Arc`, [`Mutex<T>`]).
fn decouper(t: &str) -> Option<(String, Vec<String>)> {
    let mut t = t.trim();
    loop {
        let avant = t;
        for prefixe in ["&", "mut ", "dyn ", "impl "] {
            t = t.trim_start_matches(prefixe).trim_start();
        }
        if t.starts_with('\'') {
            // une durée de vie : `&'a T`
            t = t.split_once(' ').map_or("", |(_, r)| r).trim_start();
        }
        if t == avant {
            break;
        }
    }
    let chemin: String = t.chars().take_while(|c| c.is_alphanumeric() || *c == '_' || *c == ':').collect();
    let nom = chemin.rsplit("::").next()?.to_string();
    if nom.is_empty() {
        return None;
    }
    let reste = t[chemin.len()..].trim_start();
    let mut args = Vec::new();
    if let Some(dedans) = reste.strip_prefix('<') {
        let (mut prof, mut debut) = (0usize, 0usize);
        for (i, c) in dedans.char_indices() {
            match c {
                '<' | '(' | '[' => prof += 1,
                '>' | ')' | ']' if prof > 0 => prof -= 1,
                '>' => {
                    args.push(dedans[debut..i].trim().to_string());
                    break;
                }
                ',' if prof == 0 => {
                    args.push(dedans[debut..i].trim().to_string());
                    debut = i + 1;
                }
                _ => {}
            }
        }
    }
    Some((nom, args))
}

/// Le type à travers ses enveloppes : `Arc<Mutex<T>>` reste `Mutex<T>`,
/// `MutexGuard<T>` devient `T`.
fn sans_enveloppe(t: &str) -> String {
    let mut t = t.trim().to_string();
    while let Some((nom, args)) = decouper(&t) {
        match (ENVELOPPES.contains(&nom.as_str()), args.first()) {
            (true, Some(a)) => t = a.clone(),
            _ => break,
        }
    }
    t
}

/// Le type que rend la méthode `m` appelée sur un `t`, quand il est certain.
fn methode(t: &str, m: &str) -> Option<String> {
    if let Some(enveloppe) = m.strip_prefix("wrap:") {
        return Some(format!("{enveloppe}<{t}>"));
    }
    let t = sans_enveloppe(t);
    let (nom, args) = decouper(&t)?;
    let premier = || args.first().cloned();
    match (nom.as_str(), m) {
        (_, "clone") => Some(t),
        ("Mutex", "lock") => Some(format!("LockResult<MutexGuard<{}>>", premier()?)),
        ("RwLock", "read") => Some(format!("LockResult<RwLockReadGuard<{}>>", premier()?)),
        ("RwLock", "write") => Some(format!("LockResult<RwLockWriteGuard<{}>>", premier()?)),
        ("RefCell", "borrow") => Some(format!("Ref<{}>", premier()?)),
        ("RefCell", "borrow_mut") => Some(format!("RefMut<{}>", premier()?)),
        ("Result" | "Option" | "LockResult", "unwrap" | "expect" | "?" | "await") => premier(),
        ("LockResult", _) => None,
        (c, m) if COLLECTIONS.contains(&c) && VERS_ITERATEUR.contains(&m) => Some("Iterator".into()),
        ("Iterator", m) if ADAPTATEURS.contains(&m) => Some("Iterator".into()),
        _ => None,
    }
}

/// Le type écrit d'une expression `racine(.méthode(…)|?|.await)*`, la racine
/// étant une variable typée. `None` dès qu'un maillon n'est pas certain
/// (un champ, une méthode hors de la table, une racine inconnue).
pub fn type_of_chain(expr: &str, liees: &HashMap<String, String>) -> Option<String> {
    let expr = expr.trim();
    if let Some((dedans, enveloppes, reste)) = split_constructors(expr) {
        let interieur = liees.get(dedans.trim()).cloned().or_else(|| type_construit(&dedans))?;
        let t = peel(&interieur, &enveloppes)?;
        if reste.trim().is_empty() {
            return Some(t);
        }
        let mut l = liees.clone();
        l.insert("\u{1}".into(), t);
        return type_of_chain(&format!("\u{1}{reste}"), &l);
    }
    let racine: String = expr.chars().take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '\u{1}').collect();
    let mut t = liees.get(&racine)?.clone();
    let mut reste = &expr[racine.len()..];
    loop {
        reste = reste.trim_start();
        if reste.is_empty() {
            return Some(t);
        }
        if let Some(r) = reste.strip_prefix('?') {
            t = methode(&t, "?")?;
            reste = r;
            continue;
        }
        let r = reste.strip_prefix('.')?.trim_start();
        let nom: String = r.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
        if nom.is_empty() {
            return None;
        }
        let apres = r[nom.len()..].trim_start();
        if nom == "await" {
            t = methode(&t, "await")?;
            reste = apres;
            continue;
        }
        // Un appel, avec ses arguments (turbofish compris) ; sans parenthèses,
        // c'est un champ : hors de ce palier.
        let apres = if apres.starts_with("::<") { apres.find('(').map(|i| &apres[i..])? } else { apres };
        let args = apres.strip_prefix('(')?;
        let mut prof = 1usize;
        let mut fin = None;
        for (i, c) in args.char_indices() {
            match c {
                '(' | '[' | '{' => prof += 1,
                ')' | ']' | '}' => {
                    prof -= 1;
                    if prof == 0 {
                        fin = Some(i);
                        break;
                    }
                }
                _ => {}
            }
        }
        t = methode(&t, &nom)?;
        reste = &args[fin? + 1..];
    }
}

/// **Une expression découpée en racine et chaîne** : `self.catalog.lock()
/// .unwrap()` → (`self.catalog`, [`lock`, `unwrap`]) ; `setup()?.lock()` →
/// (`setup()`, [`?`, `lock`]). La racine est un nom, un accès de champ sans
/// appel (`a.b`), ou un appel direct d'un nom (`g(…)`) ; la chaîne, des
/// appels de méthode, des `?` et des `.await`. `None` si un maillon n'en est
/// pas un (un champ après un appel, un index…).
pub fn split_chain(expr: &str) -> Option<(String, Vec<String>)> {
    let expr = expr.trim();
    let ident = |s: &str| -> usize { s.chars().take_while(|c| c.is_alphanumeric() || *c == '_').map(char::len_utf8).sum() };
    let n = ident(expr);
    if n == 0 {
        return None;
    }
    let mut racine = expr[..n].to_string();
    let mut reste = &expr[n..];
    // Des champs, tant qu'ils ne sont pas appelés.
    loop {
        let r = reste.trim_start();
        let Some(apres_point) = r.strip_prefix('.') else { break };
        let apres_point = apres_point.trim_start();
        let m = ident(apres_point);
        if m == 0 {
            break;
        }
        let suite = apres_point[m..].trim_start();
        if suite.starts_with('(') || suite.starts_with("::<") || &apres_point[..m] == "await" {
            break;
        }
        racine.push('.');
        racine.push_str(&apres_point[..m]);
        reste = &apres_point[m..];
    }
    // Ou un appel direct d'un nom.
    if !racine.contains('.') && reste.trim_start().starts_with('(') {
        let r = reste.trim_start();
        let fin = fin_des_parentheses(r)?;
        racine.push_str("()");
        reste = &r[fin + 1..];
    }
    let mut chaine = Vec::new();
    loop {
        let r = reste.trim_start();
        if r.is_empty() {
            return Some((racine, chaine));
        }
        if let Some(x) = r.strip_prefix('?') {
            chaine.push("?".to_string());
            reste = x;
            continue;
        }
        let x = r.strip_prefix('.')?.trim_start();
        let m = ident(x);
        if m == 0 {
            return None;
        }
        let nom = &x[..m];
        let suite = x[m..].trim_start();
        if nom == "await" {
            chaine.push("await".into());
            reste = suite;
            continue;
        }
        let suite = if suite.starts_with("::<") { &suite[suite.find('(')?..] } else { suite };
        if !suite.starts_with('(') {
            return None;
        }
        let fin = fin_des_parentheses(suite)?;
        chaine.push(nom.to_string());
        reste = &suite[fin + 1..];
    }
}

/// L'indice de la parenthèse qui ferme celle qui ouvre `s`.
fn fin_des_parentheses(s: &str) -> Option<usize> {
    let mut prof = 0usize;
    for (i, c) in s.char_indices() {
        match c {
            '(' | '[' | '{' => prof += 1,
            ')' | ']' | '}' => {
                prof = prof.checked_sub(1)?;
                if prof == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

/// **Un type écrit, pelé par une chaîne** : `Arc<Mutex<Catalog>>` pelé par
/// [`lock`, `unwrap`] donne `MutexGuard<Catalog>`. `None` dès qu'un maillon
/// n'est pas dans la table.
pub fn peel(t: &str, chaine: &[String]) -> Option<String> {
    let mut t = t.to_string();
    for m in chaine {
        t = methode(&t, m)?;
    }
    Some(t)
}

/// Le nom d'un type pour `qualifier_type`, à travers ses enveloppes.
pub fn type_name(t: &str) -> Option<String> {
    decouper(&sans_enveloppe(t)).map(|(nom, _)| nom)
}

/// Le type qu'un constructeur écrit nomme : `Foo::new(…)`, `Foo::default()`,
/// `Foo { … }`.
fn type_construit(expr: &str) -> Option<String> {
    let tete: String = expr.trim().chars().take_while(|c| c.is_alphanumeric() || *c == '_' || *c == ':').collect();
    let reste = expr.trim()[tete.len()..].trim_start();
    if reste.starts_with('{') && tete.rsplit("::").next()?.starts_with(char::is_uppercase) {
        return Some(tete.rsplit("::").next()?.to_string());
    }
    let (chemin, f) = tete.rsplit_once("::")?;
    let nom = chemin.rsplit("::").next()?;
    (matches!(f, "new" | "default") && nom.starts_with(char::is_uppercase) && reste.starts_with('(')).then(|| nom.to_string())
}

/// Le nom du type d'un receveur, pour `qualifier_type` : la chaîne lue, ses
/// enveloppes traversées. `None` si la chaîne ne se lit pas jusqu'au bout.
pub fn receiver_type(qualifier: &str, liees: &HashMap<String, String>) -> Option<String> {
    let t = type_of_chain(qualifier, liees)?;
    decouper(&sans_enveloppe(&t)).map(|(nom, _)| nom)
}

/// **Les variables Rust d'un scope, leur type écrit en entier, et où elles
/// valent.** Une liaison vaut sur une région du texte (octets) : un `let`
/// jusqu'à la fin de son bloc, un paramètre dans sa fonction ou sa
/// fermeture, un `Some(x)` / `Ok(x)` dans son bras de `match` ou le bloc de
/// son `if let` / `while let`. Un même nom redéclaré (`Some(catalog)` dans
/// `match catalog`) a ainsi deux types, chacun là où il vaut.
#[derive(Debug, Default, Clone)]
pub struct Bindings(HashMap<String, Vec<(usize, usize, String)>>);

impl Bindings {
    /// Les liaisons visibles à l'octet `pos` : pour chaque nom, la plus
    /// récente dont la région le contient.
    pub fn at(&self, pos: usize) -> HashMap<String, String> {
        self.0
            .iter()
            .filter_map(|(nom, l)| {
                l.iter().filter(|(debut, fin, _)| *debut <= pos && pos < *fin).max_by_key(|(debut, _, _)| *debut).map(|(_, _, t)| (nom.clone(), t.clone()))
            })
            .collect()
    }

    pub fn names(&self) -> impl Iterator<Item = &String> {
        self.0.keys()
    }

    fn lier(&mut self, nom: String, debut: usize, fin: usize, t: String) {
        self.0.entry(nom).or_default().push((debut, fin, t));
    }
}

/// Les liaisons typées d'un scope Rust : annotation d'un `let` ou d'un
/// paramètre, constructeur (`Foo::new()`, `Arc::new(Mutex::new(Foo::new()))`),
/// chaîne std sur une variable typée, élément d'une `Option` ou d'un
/// `Result` déballé par un motif.
pub fn typed_bindings_full(noeud: Node, content: &str) -> Bindings {
    let mut b = Bindings::default();
    collecter(noeud, content, &mut b);
    b
}

/// La fin de la région d'une liaison faite par `n` : son bloc, sinon son parent.
fn fin_du_bloc(n: Node) -> usize {
    let mut a = n.parent();
    while let Some(p) = a {
        if p.kind() == "block" {
            return p.end_byte();
        }
        a = p.parent();
    }
    n.parent().map_or(n.end_byte(), |p| p.end_byte())
}

fn fonction_englobante(n: Node) -> (usize, usize) {
    let mut a = n.parent();
    while let Some(p) = a {
        if matches!(p.kind(), "function_item" | "closure_expression") {
            return (p.start_byte(), p.end_byte());
        }
        a = p.parent();
    }
    (n.start_byte(), n.end_byte())
}

/// `Some(x)` ou `Ok(x)` : le nom lié et le constructeur.
fn motif_deballant<'a>(motif: Node<'a>, content: &str) -> Option<(String, &'static str)> {
    let mut m = motif;
    while m.kind() == "match_pattern" {
        m = m.named_child(0)?;
    }
    if m.kind() != "tuple_struct_pattern" {
        return None;
    }
    let ctor = match texte(m.child_by_field_name("type")?, content).rsplit("::").next()? {
        "Some" => "Some",
        "Ok" => "Ok",
        _ => return None,
    };
    let mut c = m.walk();
    let liees: Vec<Node> = m.named_children(&mut c).skip(1).collect();
    let [seul] = liees.as_slice() else { return None };
    simple_name(*seul, content).map(|n| (n, ctor))
}

/// Le type de l'élément que `Some` / `Ok` déballe d'un `t`.
fn element(t: &str, ctor: &str) -> Option<String> {
    let (nom, args) = decouper(t)?;
    match (nom.as_str(), ctor) {
        ("Option", "Some") | ("Result", "Ok") => args.first().cloned(),
        _ => None,
    }
}

fn collecter(n: Node, content: &str, b: &mut Bindings) {
    match n.kind() {
        "let_declaration" => {
            if let Some(nom) = n.child_by_field_name("pattern").and_then(|m| simple_name(m, content)) {
                let valeur = n.child_by_field_name("value");
                let connus = b.at(valeur.map_or(n.start_byte(), |v| v.start_byte()));
                let type_ = n
                    .child_by_field_name("type")
                    .map(|t| texte(t, content).to_string())
                    .or_else(|| valeur.and_then(|v| type_of_chain(texte(v, content), &connus)))
                    .or_else(|| valeur.and_then(|v| initializer_type(v, content)).filter(|t| !is_wrapper(t)));
                if let Some(t) = type_ {
                    b.lier(nom, n.end_byte(), fin_du_bloc(n), t);
                }
            }
        }
        "parameter" => {
            if let (Some(nom), Some(t)) = (n.child_by_field_name("pattern").and_then(|m| simple_name(m, content)), n.child_by_field_name("type")) {
                let (debut, fin) = fonction_englobante(n);
                b.lier(nom, debut, fin, texte(t, content).to_string());
            }
        }
        "match_expression" => {
            if let (Some(valeur), Some(corps)) = (n.child_by_field_name("value"), n.child_by_field_name("body")) {
                if let Some(t) = b.at(valeur.start_byte()).get(texte(valeur, content).trim()).cloned() {
                    let mut c = corps.walk();
                    for bras in corps.named_children(&mut c).filter(|x| x.kind() == "match_arm") {
                        if let Some((nom, ctor)) = bras.child_by_field_name("pattern").and_then(|m| motif_deballant(m, content)) {
                            if let Some(e) = element(&t, ctor) {
                                b.lier(nom, bras.start_byte(), bras.end_byte(), e);
                            }
                        }
                    }
                }
            }
        }
        "if_expression" | "while_expression" => {
            let condition = n.child_by_field_name("condition").filter(|c| c.kind() == "let_condition");
            let bloc = n.child_by_field_name("consequence").or_else(|| n.child_by_field_name("body"));
            if let (Some(cond), Some(bloc)) = (condition, bloc) {
                if let (Some(motif), Some(valeur)) = (cond.child_by_field_name("pattern"), cond.child_by_field_name("value")) {
                    if let (Some((nom, ctor)), Some(t)) = (motif_deballant(motif, content), b.at(valeur.start_byte()).get(texte(valeur, content).trim()).cloned()) {
                        if let Some(e) = element(&t, ctor) {
                            b.lier(nom, bloc.start_byte(), bloc.end_byte(), e);
                        }
                    }
                }
            }
        }
        _ => {}
    }
    let mut c = n.walk();
    for enfant in n.named_children(&mut c) {
        collecter(enfant, content, b);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn liees(paires: &[(&str, &str)]) -> HashMap<String, String> {
        paires.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect()
    }

    #[test]
    fn un_verrou_deballe_rend_le_type_garde() {
        let l = liees(&[("catalog", "Arc<Mutex<Catalog>>"), ("r", "std::sync::RwLock<Index>"), ("c", "&RefCell<Etat>")]);
        assert_eq!(receiver_type("catalog.lock().unwrap()", &l).as_deref(), Some("Catalog"));
        assert_eq!(receiver_type("catalog.lock().expect(\"verrou\")", &l).as_deref(), Some("Catalog"));
        assert_eq!(receiver_type("r.read().unwrap()", &l).as_deref(), Some("Index"));
        assert_eq!(receiver_type("c.borrow_mut()", &l).as_deref(), Some("Etat"));
        // `lock()` seul rend un `LockResult` : pas de méthode du projet dessus.
        assert_eq!(receiver_type("catalog.lock()", &l).as_deref(), Some("LockResult"));
    }

    #[test]
    fn une_collection_rend_un_iterateur() {
        let l = liees(&[("v", "Vec<Node>"), ("m", "&HashMap<String, Vec<u8>>")]);
        assert_eq!(receiver_type("v.iter().map(|x| x.id).filter(|x| *x > 0)", &l).as_deref(), Some("Iterator"));
        assert_eq!(receiver_type("m.values()", &l).as_deref(), Some("Iterator"));
        assert_eq!(receiver_type("v", &l).as_deref(), Some("Vec"));
    }

    #[test]
    fn un_maillon_incertain_arrete_la_lecture() {
        let l = liees(&[("v", "Vec<Node>"), ("n", "Node")]);
        // Une méthode du projet, un champ, une racine inconnue : rien.
        assert_eq!(receiver_type("n.enfants()", &l), None);
        assert_eq!(receiver_type("v.len", &l), None);
        assert_eq!(receiver_type("inconnu.lock().unwrap()", &l), None);
        assert_eq!(receiver_type("v.iter().collect::<Vec<_>>()", &l), None);
    }

    #[test]
    fn une_expression_se_decoupe_en_racine_et_chaine() {
        let v = |s: &[&str]| s.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        assert_eq!(split_chain("self.catalog.lock().unwrap()"), Some(("self.catalog".into(), v(&["lock", "unwrap"]))));
        assert_eq!(split_chain("setup()?.lock()"), Some(("setup()".into(), v(&["?", "lock"]))));
        assert_eq!(split_chain("catalog\n    .lock()\n    .unwrap()"), Some(("catalog".into(), v(&["lock", "unwrap"]))));
        assert_eq!(split_chain("guard"), Some(("guard".into(), vec![])));
        assert_eq!(split_chain("a.b().c"), None, "un champ après un appel");
        assert_eq!(peel("Arc<Mutex<Catalog>>", &v(&["lock", "unwrap"])).as_deref(), Some("MutexGuard<Catalog>"));
        assert_eq!(type_name("MutexGuard<Catalog>").as_deref(), Some("Catalog"));
    }

    #[test]
    fn un_constructeur_enveloppe_son_contenu() {
        let l = liees(&[("c", "Catalog")]);
        assert_eq!(split_constructors("Arc::new(Mutex::new(catalogue(4)))"), Some(("catalogue(4)".into(), vec!["wrap:Mutex".into(), "wrap:Arc".into()], "".into())));
        assert_eq!(receiver_type("Arc::new(Mutex::new(c)).lock().unwrap()", &l).as_deref(), Some("Catalog"));
        assert_eq!(type_of_chain("std::sync::Arc::new(Mutex::new(Foo::new()))", &l).as_deref(), Some("Arc<Mutex<Foo>>"));
        assert!(is_wrapper("Arc") && !is_wrapper("Catalog"));
    }

    #[test]
    fn decouper_lit_les_generiques_de_premier_niveau() {
        assert_eq!(decouper("&mut std::sync::Arc<Mutex<T>>"), Some(("Arc".into(), vec!["Mutex<T>".into()])));
        assert_eq!(decouper("HashMap<String, Vec<(u8, u8)>>"), Some(("HashMap".into(), vec!["String".into(), "Vec<(u8, u8)>".into()])));
        assert_eq!(decouper("&'a Node"), Some(("Node".into(), vec![])));
    }
}
