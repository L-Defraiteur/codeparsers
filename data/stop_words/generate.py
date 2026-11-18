#!/usr/bin/env python3
"""Generate data/stop_words/<language>.toml from the pinned local standard library documentation.

Usage: python3 data/stop_words/generate.py <rust|python|c> > data/stop_words/<language>.toml
"""
import glob
import json
import keyword
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


def python():
    # Hard keywords only: a soft keyword (match, case, type, _) is a legal identifier.
    version = sys.version.split()[0]
    return f"le module keyword de CPython {version} : kwlist (sans les mots-clés doux softkwlist)", set(keyword.kwlist)


C_GRAMMAR = "tree-sitter-c-0.24.2"


def c():
    # The words the C grammar itself reserves: its anonymous alphabetic tokens, its primitive types,
    # and its true, false and null literals. The grammar is the crate codeparsers parses with.
    src = glob.glob(os.path.expanduser(f"~/.cargo/registry/src/*/{C_GRAMMAR}/src"))[0]
    with open(f"{src}/node-types.json", encoding="utf-8") as stream:
        nodes = json.load(stream)
    names = {n["type"] for n in nodes if not n["named"] and re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", n["type"])}
    with open(f"{src}/grammar.json", encoding="utf-8") as stream:
        rules = json.load(stream)["rules"]

    def strings(rule):
        if isinstance(rule, dict):
            if rule.get("type") == "STRING":
                yield rule["value"]
            for value in rule.values():
                yield from strings(value)
        elif isinstance(rule, list):
            for value in rule:
                yield from strings(value)

    for rule in ["primitive_type", "true", "false", "null"]:
        names |= set(strings(rules[rule]))
    return f"la grammaire {C_GRAMMAR} : ses jetons alphabétiques anonymes, primitive_type, true, false et null", names


language = sys.argv[1]
source, names = {"rust": rust, "python": python, "c": c}[language]()
print(f"# Les mots vides de {language} (codeparsers, `scope_extraction::standard`).")
print("# Généré par data/stop_words/generate.py — ne pas éditer à la main : relancer le générateur.")
print("# Un identifiant de cette liste n'est pas une référence.")
print(f'language = "{language}"')
print(f'source = "{source}"')
print("names = [")
for name in sorted(names):
    print(f'    "{name}",')
print("]")
