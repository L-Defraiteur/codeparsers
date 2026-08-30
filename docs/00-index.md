# Les documents de codeparsers

| document | de quoi ça parle |
|---|---|
| [30 août 2026 — cahier des charges](30-aout-2026-06h00/01-cahier-des-charges-tout-le-fichier-est-couvert.md) | l'union des scopes d'un fichier doit couvrir le fichier entier ; ce qu'on n'a pas compris doit le dire au lieu d'être jeté |
| [30 août 2026 — comment on saura que ça marche](30-aout-2026-06h00/03-comment-on-saura-que-ca-marche.md) | les critères de réussite du chantier de couverture, écrits d'avance |
| [30 août 2026 — ce que la mesure dit](30-aout-2026-06h00/05-ce-que-la-mesure-dit.md) | la ligne de base : 17 % du dépôt n'était pas compris, 69 fichiers sur 1 654 le disaient |
| [22 février 2026](22-fevrier-2026-15h25/) | suppression d'`UniversalScope`, couverture Go et TypeScript, closures Rust et `virtual` C++ |
| [21 février 2026](21-fevrier-2026-23h03/) | finalisation du portage depuis TypeScript, comparaison des deux sorties, premiers écarts de couverture |

## Ce qui vit ailleurs

Deux documents parlent de la couverture sans parler du parseur, et sont restés
dans [rag3weaver](https://github.com/L-Defraiteur/rag3db), qui **consomme**
codeparsers :

- `docs/30-aout-2026-06h00/02-ce-que-ca-change-en-aval.md` — ce que la
  couverture change dans l'ingestion, le découpage et la recherche.
- `docs/30-aout-2026-06h00/06-ou-on-en-est.md` — le rapport de la session qui a
  mesuré, corrigé `.h`, et sorti ce dépôt.

La règle : ce qui décrit **le parsage** vit ici, ce qui décrit **la couture**
vit chez le consommateur. Un cahier des charges qui ne voyage pas avec sa mise
en œuvre diverge.
