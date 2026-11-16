#!/usr/bin/env python3
"""Regénère `data/stop_words/<language>.toml` depuis le poste, jamais à la main (rag3db, ticket « la liste des
names vides de Rust est une copie de celle de JavaScript », 16 novembre 2026).

- Rust : les pages `keyword.<word>.html` de la doc de std de la chaîne épinglée (composant `rust-docs`) —
  `SelfTy` y nomme `Self` —, plus les variantes du prélude, lues sur les pages d'`Option` et de `Result`
  (ancres `variant.<nom>`).

Usage : python3 data/stop_words/generate.py rust > data/stop_words/rust.toml
"""
import os, re, sys

TOOLCHAIN = "1.98.1"
# The weak keywords of the Rust reference (a keyword only in a given place, and a legal identifier
# elsewhere: `HashSet::union`, a project's `fn union`) are not stop words. Of them, only `union` has a
# keyword page in the std doc.
WEAK = {"union"}
DOC = os.path.expanduser(f"~/.rustup/toolchains/{TOOLCHAIN}-x86_64-unknown-linux-gnu/share/doc/rust/html/std")


def rust():
    names = set()
    for f in os.listdir(DOC):
        m = re.fullmatch(r"keyword\.([A-Za-z_]+)\.html", f)
        if m:
            word = "Self" if m.group(1) == "SelfTy" else m.group(1)
            if word not in WEAK:
                names.add(word)
    for page in ["option/enum.Option.html", "result/enum.Result.html"]:
        names |= set(re.findall(r'id="variant\.([A-Za-z]+)"', open(f"{DOC}/{page}", encoding="utf-8").read()))
    return f"la doc de std de rustc {TOOLCHAIN} : les pages keyword.* (sans les names-clés faibles : union), et les variantes d'Option et de Result", names


language = sys.argv[1]
source, names = {"rust": rust}[language]()
print(f"# Les names vides de {language} (codeparsers, `scope_extraction::standard`).")
print("# Généré par data/stop_words/generate.py — ne pas éditer à la main : relancer le générateur.")
print("# Un identifiant de cette liste n'est pas une référence.")
print(f'language = "{language}"')
print(f'source = "{source}"')
print("names = [")
for n in sorted(names):
    print(f'    "{n}",')
print("]")
