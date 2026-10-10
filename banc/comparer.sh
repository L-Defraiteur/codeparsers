#!/bin/bash
# Le banc avant / après, sur le même corpus.
#
# Bâtit le banc de couverture à un commit de référence (par défaut
# origin/master) et à l'état courant, les fait tourner tous deux sur les
# mêmes fichiers (les dépôts tels qu'ils sont sur le disque), et échoue si
# une mesure recule dans son mauvais sens. Les références commitées dans
# banc/ disent l'état publié ; ce script dit ce qu'un changement de
# codeparsers fait, corpus égal — un dépôt qui grossit ne fait pas reculer
# une ligne à tort.
#
# Usage : banc/comparer.sh [commit] [--ecrire]
#   RAG3DB : racine du dépôt rag3db (défaut ../rag3db)
#   CIBLE  : dossier des targets cargo (défaut ~/.cache/rag3weaver-build/codeparsers-banc)
set -u
ICI=$(cd "$(dirname "$0")/.." && pwd)
REF=${1:-origin/master}
ECRIRE=0; [ "${2:-}" = "--ecrire" ] || [ "${1:-}" = "--ecrire" ] && ECRIRE=1
[ "$REF" = "--ecrire" ] && REF=origin/master
RAG3DB=${RAG3DB:-$ICI/../rag3db}
CIBLE=${CIBLE:-$HOME/.cache/rag3weaver-build/codeparsers-banc}
AVANT=$CIBLE/avant-src
SORTIES=$CIBLE/sorties
mkdir -p "$SORTIES"
rm -rf "$AVANT"
git -C "$ICI" worktree add -q --detach "$AVANT" "$REF" || exit 2
trap 'git -C "$ICI" worktree remove --force "$AVANT"' EXIT
# Le banc d'aujourd'hui, compilé contre le codeparsers d'avant : seule la
# bibliothèque change entre les deux binaires.
cp "$ICI/examples/banc_couverture.rs" "$AVANT/examples/banc_couverture.rs"
( cd "$AVANT" && CARGO_TARGET_DIR=$CIBLE/avant cargo build --release -j8 -q --example banc_couverture ) || exit 2
( cd "$ICI" && CARGO_TARGET_DIR=$CIBLE/apres cargo build --release -j8 -q --example banc_couverture ) || exit 2
A=$CIBLE/avant/release/examples/banc_couverture
B=$CIBLE/apres/release/examples/banc_couverture
echec=0
while read -r nom racines; do
  [ -z "$nom" ] && continue
  eval "set -- $racines"
  "$A" "$nom" "$@" --ecrire "$SORTIES/$nom.avant.txt" > /dev/null 2>&1
  if "$B" "$nom" "$@" --verifier "$SORTIES/$nom.avant.txt" > "$SORTIES/$nom.comparaison.txt" 2>&1; then r=ok; else r=RECUL; echec=1; fi
  echo "$nom : $r — $(grep -c '^monte' "$SORTIES/$nom.comparaison.txt") monte, $(grep -c '^RECUL' "$SORTIES/$nom.comparaison.txt") recul(s)"
  grep '^RECUL' "$SORTIES/$nom.comparaison.txt"
  [ $ECRIRE = 1 ] && "$B" "$nom" "$@" --ecrire "$ICI/banc/$nom.txt" > /dev/null 2>&1
done <<LISTE
rag3db-src $RAG3DB/src
rag3db-test $RAG3DB/test
rag3weaver $RAG3DB/extension/rag3weaver/src $RAG3DB/extension/rag3weaver/tests
codeparsers $ICI/src $ICI/tests
ir $RAG3DB/extension/rag3weaver/ir
LISTE
exit $echec
