#!/usr/bin/env python3
"""Generate data/stop_words/<language>.toml from the pinned local standard library documentation.

Usage: python3 data/stop_words/generate.py rust > data/stop_words/rust.toml
"""
import os
import re
import sys

TOOLCHAIN = "1.98.1"
WEAK_KEYWORDS = {"union"}
DOC = os.path.expanduser(f"~/.rustup/toolchains/{TOOLCHAIN}-x86_64-unknown-linux-gnu/share/doc/rust/html/std")


def rust():
    names = set()
    for filename in os.listdir(DOC):
        match = re.fullmatch(r"keyword\.([A-Za-z_]+)\.html", filename)
        if match:
            word = "Self" if match.group(1) == "SelfTy" else match.group(1)
            if word not in WEAK_KEYWORDS:
                names.add(word)
    for page in ["option/enum.Option.html", "result/enum.Result.html"]:
        with open(f"{DOC}/{page}", encoding="utf-8") as stream:
            names |= set(re.findall(r'id="variant\.([A-Za-z]+)"', stream.read()))
    source = f"la doc de std de rustc {TOOLCHAIN} : les pages keyword.* (sans les mots-clés faibles : union), et les variantes d'Option et de Result"
    return source, names


language = sys.argv[1]
source, names = {"rust": rust}[language]()
print(f"# Les mots vides de {language} (codeparsers, `scope_extraction::standard`).")
print("# Généré par data/stop_words/generate.py — ne pas éditer à la main : relancer le générateur.")
print("# Un identifiant de cette liste n'est pas une référence.")
print(f'language = "{language}"')
print(f'source = "{source}"')
print("names = [")
for name in sorted(names):
    print(f'    "{name}",')
print("]")
