# Le banc de couverture

Une ligne de référence par dépôt : ce que codeparsers comprend de nos
dépôts, et ce qu'il ne sait pas comprendre. Le témoin du chantier
« indexer nos propres dépôts » (10 octobre 2026).

| Fichier | Dépôt mesuré |
|---|---|
| `rag3db-src.txt` | `rag3db/src` (C++) |
| `rag3db-test.txt` | `rag3db/test` (C++) |
| `rag3weaver.txt` | `rag3db/extension/rag3weaver/src` et `tests` (Rust) |
| `codeparsers.txt` | ce dépôt, `src` et `tests` (Rust) |
| `ir.txt` | `rag3db/extension/rag3weaver/ir` (Rust, l'IR des requêtes) |

## Les mesures

- `fichiers.<langue>`, `octets.<langue>.ko` : ce qui est analysé.
- `analyse.<langue>.erreur_pct` : la part des octets sous un nœud ERROR de
  tree-sitter. **Doit descendre.**
- `scopes.<genre>` : les scopes par genre. Neutre : se relit.
- `<mode>.refs.<usage>` et `<mode>.refs.<usage>.relies_pct` : les références
  d'un scope (une par scope et par nom, builtins exceptés), par genre
  d'usage, et la part qui devient une relation. **Doit monter.**
  `fichier_seul` est le mode de rag3weaver (les liens entre fichiers y sont
  l'affaire de son rendez-vous par le nom) ; `entre_fichiers` dit ce que
  codeparsers relie seul en voyant tout le dépôt.
- `<mode>.non_resolues`, `.raison.*`, `.genre.*` : le relevé des références
  non résolues (`RelationshipResolutionResult::unresolved_references`), par
  raison et par genre du scope qui les porte. **Doit descendre.**

## Rejouer : avant / après, sur le même corpus

```
banc/comparer.sh [commit]          # défaut : origin/master
banc/comparer.sh [commit] --ecrire # et réécrire les références
```

Bâtit le banc au commit de référence et à l'état courant, les fait tourner
sur les mêmes fichiers, et échoue si une mesure recule. C'est la
vérification d'un lot : nos dépôts changent (rag3db et rag3weaver avec les
autres sessions, codeparsers se mesure lui-même), une ligne de référence
fixe ne juge que sur un corpus fixe. Les fichiers de `banc/` disent l'état
publié, réécrits avec `--ecrire` dans le commit du lot.

## Un dépôt à la main



```
cargo run --release --example banc_couverture -- rag3db-src ../rag3db/src --verifier banc/rag3db-src.txt
```

`--verifier` échoue si une mesure a reculé dans son mauvais sens ; elle
imprime aussi les vingt fichiers aux plus nombreux appels non résolus.
`--trous` ajoute le rapport des trous : chaque non-résolue rangée par forme
(méthode sur un receveur, appel nu, appel par chemin, lecture de champ,
macro, `std::`, import, type, nom seul), avec son coût pour les outils
(un trou dans `storage/` ou `transaction/` pèse double) et le lot qui la
fermerait.

`--ecrire` réécrit la référence : à faire dans le commit qui fait monter
une ligne, jamais pour en effacer une qui descend.
