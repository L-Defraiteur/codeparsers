# codeparsers

Extraction de scopes — fonctions, classes, méthodes, modules — depuis du code
source, par tree-sitter. Rend des `ScopeInfo` avec leurs positions en lignes et
en octets, leurs signatures, leurs paramètres, et les références qu'ils font
les uns aux autres.

Écrit pour [rag3db](https://github.com/L-Defraiteur/rag3db), où il alimente
l'ingestion d'un dépôt dans un graphe interrogeable.

## Ce qu'il parse

**Par grammaire tree-sitter** — TypeScript, JavaScript, Python, Rust, Go, C,
C++, C#. Chaque langage a son extracteur (`src/scope_extraction/`), au-dessus
d'une base commune qui fait le gros du travail.

**Par heuristique** — un parseur générique (`src/generic/`) qui suit les
accolades ou l'indentation et reconnaît les mots-clés usuels (`fn`, `def`,
`func`, `class`, `struct`…). Chaque scope qu'il rend porte un **score de
confiance**, parce qu'une conjecture qui ne s'avoue pas est un mensonge.

**Par format** — Markdown, CSS, SCSS, Vue, Svelte.

À côté : la résolution des imports vers des chemins réels
(`src/import_resolution/`), la résolution des relations entre scopes
(`src/relationship_resolution/`), et `src/shell.rs`, qui réduit une ligne de
commande en `argv` pour qu'une politique puisse la juger — il produit des
faits, il ne juge rien.

## La sonde de couverture

`examples/couverture.rs` mesure ce que les parseurs comprennent **vraiment**,
par opposition à ce qu'ils déclarent :

```sh
cargo run --example couverture -- <racine>...      # le tableau par extension
DETAIL=1 cargo run --example couverture -- ...     # la 1re erreur de chaque fichier
FORCER=cpp cargo run --example couverture -- ...   # .c/.h avec la grammaire C++
```

Elle rend, par extension : le pavage du fichier par les scopes de premier
niveau, les trous et les recouvrements, et — indépendamment de ce que le
parseur prétend — les octets réellement sous un nœud `ERROR` de tree-sitter.

Elle existe parce que l'écart est grand. Sur un dépôt de 3 918 fichiers :
1 654 ont un arbre en erreur, **69** le déclarent, et 17 % des octets sont
sous un `ERROR`. Un fichier dont 99 % du texte n'est pas compris peut sortir
413 scopes et 100 % de pavage sans qu'une seule ligne le signale.

C'est le défaut que la sonde rend visible, et qu'il reste à corriger : un
moteur de recherche qui perd du texte sans le dire ment par omission.

## Limites connues

- `validate_ast` teste `root.kind() != "ERROR"`, et la racine d'un arbre
  tree-sitter est `source_file` ou `translation_unit` : le champ `ast_valid`
  est structurellement vrai. Ne pas s'y fier ; utiliser la sonde.
- `.h` est mappé sur la grammaire C. Pour un projet C++, c'est le mauvais
  choix — la sonde mesure l'écart, qui est large.
- Le passage aux grammaires tree-sitter est figé en **0.23**, pour l'ABI :
  0.25 est compilée en ABI 15 et tree-sitter 0.24 refuse au-delà de 14. Cargo
  laisse passer, l'exécution non.
- Les parseurs non-code (Markdown, CSS, Vue, Svelte, générique) sont complets
  mais aucune extension ne les atteint depuis `EXTENSION_TO_PARSER`.

## Licence

MIT. Voir [LICENSE](LICENSE).
