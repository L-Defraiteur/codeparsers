#!/usr/bin/env python3
"""Regénère `data/std_methods/<langue>.toml` depuis la bibliothèque standard installée sur le poste,
jamais à la main (rag3db, ticket « la marque nom relie un appel de méthode standard à un homonyme »).

- Rust : les ancres `method.<nom>` et `tymethod.<nom>` des pages de la doc de std de la chaîne
  épinglée (composant `rust-docs`), pour les types courants de `TYPES_RUST`.
- C++ : les `CXXMethodDecl` publics des classes de `TYPES_CPP`, lus dans l'AST de clang sur
  libstdc++ (`clang++ -std=c++20 -Xclang -ast-dump=json`).
- Python : `dir()` des types de `TYPES_PYTHON` de l'interpréteur, sans les noms à double souligné.

Usage : python3 data/std_methods/generer.py <rust|cpp|python> > data/std_methods/<langue>.toml
"""
import json, os, re, subprocess, sys

TOOLCHAIN = "1.98.1"
DOC = os.path.expanduser(f"~/.rustup/toolchains/{TOOLCHAIN}-x86_64-unknown-linux-gnu/share/doc/rust/html/std")
TYPES_RUST = [
    "primitive.str", "primitive.slice", "primitive.char", "primitive.bool", "primitive.u8", "primitive.u32", "primitive.u64",
    "primitive.usize", "primitive.i32", "primitive.i64", "primitive.f32", "primitive.f64",
    "string/struct.String", "vec/struct.Vec", "option/enum.Option", "result/enum.Result", "boxed/struct.Box",
    "collections/struct.HashMap", "collections/struct.HashSet", "collections/struct.BTreeMap", "collections/struct.BTreeSet",
    "collections/struct.VecDeque", "collections/struct.BinaryHeap",
    "collections/hash_map/enum.Entry", "collections/btree_map/enum.Entry",
    "iter/trait.Iterator", "iter/trait.DoubleEndedIterator", "iter/trait.ExactSizeIterator", "iter/struct.Peekable",
    "time/struct.Duration", "time/struct.Instant", "time/struct.SystemTime",
    "path/struct.Path", "path/struct.PathBuf", "ffi/struct.OsStr", "ffi/struct.OsString",
    "sync/struct.Mutex", "sync/struct.MutexGuard", "sync/struct.RwLock", "sync/struct.Arc", "sync/struct.Condvar",
    "sync/struct.OnceLock", "sync/atomic/struct.Atomic",
    "sync/mpsc/struct.Sender", "sync/mpsc/struct.Receiver", "sync/mpsc/struct.SyncSender",
    "rc/struct.Rc", "cell/struct.RefCell", "cell/struct.Cell", "thread/struct.JoinHandle",
    "fs/struct.File", "io/trait.Read", "io/trait.Write", "io/trait.BufRead",
    "process/struct.Command", "process/struct.Child", "fmt/struct.Formatter",
    "net/struct.TcpStream", "net/struct.TcpListener",
]
TYPES_CPP = {
    "basic_string", "basic_string_view", "vector", "deque", "list", "array", "map", "multimap", "set", "unordered_map",
    "unordered_set", "optional", "shared_ptr", "unique_ptr", "weak_ptr", "pair", "tuple", "function", "duration",
    "time_point", "atomic", "mutex", "shared_mutex", "unique_lock", "lock_guard", "condition_variable", "thread",
    "basic_ostream", "basic_istream", "basic_ostringstream", "basic_stringstream", "span", "variant", "bitset", "path",
    "__atomic_base",
}
EN_TETES_CPP = ["string", "string_view", "vector", "deque", "list", "array", "map", "set", "unordered_map", "unordered_set",
                "optional", "memory", "utility", "tuple", "functional", "chrono", "atomic", "mutex", "shared_mutex",
                "condition_variable", "thread", "iostream", "sstream", "span", "variant", "bitset", "filesystem"]
TYPES_PYTHON = ["str", "bytes", "bytearray", "list", "dict", "set", "frozenset", "tuple", "int", "float",
                "io.TextIOWrapper", "io.BufferedReader", "pathlib.Path", "re.Pattern", "re.Match",
                "subprocess.Popen", "collections.deque", "collections.OrderedDict", "collections.defaultdict",
                "threading.Thread", "threading.Lock", "queue.Queue", "datetime.datetime", "datetime.timedelta"]


def rust():
    noms, vus = set(), []
    for t in TYPES_RUST:
        texte = open(f"{DOC}/{t}.html", encoding="utf-8").read()
        noms |= set(re.findall(r'id="(?:ty)?method\.([a-z_][a-z0-9_]*)"', texte))
        vus.append(t)
    return f"la doc de std de rustc {TOOLCHAIN} (composant rust-docs), ancres method.* et tymethod.*", vus, noms


def cpp():
    src = "".join(f"#include <{h}>\n" for h in EN_TETES_CPP)
    ast = subprocess.run(["clang++", "-std=c++20", "-fsyntax-only", "-Xclang", "-ast-dump=json", "-x", "c++", "-"],
                         input=src, capture_output=True, text=True, check=True).stdout
    noms = set()

    def parcourir(n, classe):
        k = n.get("kind")
        if k in ("CXXRecordDecl", "ClassTemplateDecl", "ClassTemplateSpecializationDecl", "ClassTemplatePartialSpecializationDecl"):
            classe = n.get("name", classe)
        if k == "CXXMethodDecl" and classe in TYPES_CPP:
            nom = n.get("name", "")
            if re.fullmatch(r"[a-z][a-z0-9_]*", nom) and n.get("access", "public") == "public":
                noms.add(nom)
        for c in n.get("inner", []) or []:
            parcourir(c, classe)

    parcourir(json.loads(ast), None)
    version = subprocess.run(["clang++", "--version"], capture_output=True, text=True).stdout.splitlines()[0]
    return f"l'AST de {version} sur libstdc++ (-std=c++20), méthodes publiques", sorted(TYPES_CPP), noms


def python():
    import importlib
    noms = set()
    for t in TYPES_PYTHON:
        if "." in t:
            mod, nom = t.rsplit(".", 1)
            obj = getattr(importlib.import_module(mod), nom)
        else:
            obj = __builtins__.__dict__[t] if hasattr(__builtins__, "__dict__") else __builtins__[t]
        noms |= {n for n in dir(obj) if not n.startswith("_") and callable(getattr(obj, n, None))}
    return f"dir() des types de Python {sys.version.split()[0]}, sans les noms soulignés", TYPES_PYTHON, noms


langue = sys.argv[1]
source, types, noms = {"rust": rust, "cpp": cpp, "python": python}[langue]()
print(f"# Les méthodes de la bibliothèque standard de {langue} (codeparsers, `scope_extraction::std_methods`).")
print("# Généré par data/std_methods/generer.py — ne pas éditer à la main : relancer le générateur.")
print("# Un appel sur une locale sans type lu dont le nom est ici ne se relie pas par le nom entre fichiers.")
print(f'langue = "{langue}"')
print(f'source = "{source}"')
print("types = [" + ", ".join(json.dumps(t) for t in types) + "]")
print("noms = [")
for n in sorted(noms):
    print(f'    "{n}",')
print("]")
