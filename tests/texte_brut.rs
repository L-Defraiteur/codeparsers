//! **Un fichier sans grammaire est rendu entier, ou il n'est pas rendu.**
//!
//! L'invariant du cahier des charges du 30 août, dans son cas le plus simple :
//! l'union des scopes couvre le fichier, sans trou et sans recouvrement.

use codeparsers::parallel::parser_worker::analyser_texte_brut;
use codeparsers::scope_extraction::types::ScopeInfoType;

fn couvre_tout(contenu: &str) {
    let a = analyser_texte_brut("notes.md", contenu);
    assert_eq!(a.scopes.len(), 1, "un fichier sans parseur rend un scope, pas zéro ni deux");
    let s = &a.scopes[0];
    assert_eq!(s.scope_start_byte, 0, "le scope commence au premier octet");
    assert_eq!(s.scope_end_byte, contenu.len(), "le scope finit au dernier octet");
    assert_eq!(s.content, contenu, "le contenu est le texte, tel quel");
    assert_eq!(a.octets, contenu.len(), "les octets déclarés sont ceux du fichier");
}

#[test]
fn le_scope_unique_couvre_le_fichier() {
    couvre_tout("# Titre\n\nDeux lignes de prose.\n");
    couvre_tout("une seule ligne sans retour");
    couvre_tout("");
    couvre_tout("\n\n\n");
    // Les offsets sont en octets, pas en caractères : un accent ne doit pas
    // décaler la fin du scope.
    couvre_tout("héhé — des octets multi-octets\n");
}

#[test]
fn le_genre_dit_qu_on_n_a_pas_essaye() {
    let a = analyser_texte_brut("config.toml", "clef = \"valeur\"\n");
    assert!(matches!(a.scopes[0].r#type, ScopeInfoType::TexteBrut));
    assert!(a.aucun_parseur, "aucune grammaire n'a été tentée, et ça se dit");
    // `ast_valid` parle d'une tentative qui a eu lieu : ici il n'y en a pas
    // eu, donc rien n'a échoué. Confondre les deux effacerait la distinction
    // que tout le chantier de couverture cherche à établir.
    assert!(a.ast_valid, "on n'a pas échoué à parser — on n'a pas essayé");
    assert!(a.ast_issues.is_empty());
}

#[test]
fn le_scope_porte_un_nom_citable() {
    let a = analyser_texte_brut("/un/chemin/long/README.md", "a\nb\nc\n");
    // Un scope sans nom ne se cite pas : celui-ci porte son fichier et son
    // étendue, pas le chemin entier.
    assert_eq!(a.scopes[0].name, "README.md:1-3");
}

#[test]
fn rien_n_est_inventé() {
    let a = analyser_texte_brut("script.sh", "#!/bin/sh\necho salut\n");
    let s = &a.scopes[0];
    assert!(s.signature.is_empty(), "il n'y a pas de signature, en inventer une serait mentir");
    assert!(s.parameters.is_empty());
    assert!(s.imports.is_empty());
    assert!(s.identifier_references.is_empty());
    assert!(s.children.is_empty());
    assert_eq!(s.depth, 0);
    assert!(s.parent.is_none());
}

#[test]
fn le_hash_de_contenu_est_rempli() {
    let a = analyser_texte_brut("notes.md", "du texte\n");
    let b = analyser_texte_brut("autre.md", "du texte\n");
    assert!(a.content_hash.is_some(), "le hash sert d'empreinte au `stale` par fichier");
    assert_eq!(a.content_hash, b.content_hash, "il porte sur le contenu, pas sur le chemin");
}
