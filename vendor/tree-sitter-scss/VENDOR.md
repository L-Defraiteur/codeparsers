# tree-sitter-scss 1.0.0, copie locale

Source : crates.io `tree-sitter-scss` 1.0.0 (avril 2024), dépôt
https://github.com/tree-sitter-grammars/tree-sitter-scss, auteur Amaan
Qureshi, licence MIT. Seule version publiée.

Une seule modification, dans `bindings/rust/build.rs` : `-Wno-unused-parameter`
passe par `flag_if_supported` au lieu de `flag` (MSVC le refusait, D8021, au
bâti Windows de rag3weaver), et `-utf-8` est ajouté pour une cible MSVC — la
forme de tree-sitter-css 0.23.2. Rien n'est proposé à l'amont.

Le reste (grammaire, `src/parser.c`, `src/scanner.c`, requêtes) est tel que
publié. Pour reprendre une version amont : recopier la crate du registre,
puis réappliquer ce changement.
