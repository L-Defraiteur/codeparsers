#!/usr/bin/env python3
"""Generate data/stop_words/<language>.toml from the pinned local standard library documentation.

Usage: python3 data/stop_words/generate.py <rust|python|c|cpp|csharp|go|javascript> > data/stop_words/<language>.toml

Each language gives two lists: its stop words (`names`: a word of the language, never a reference) and its built-in
names (`builtins`: names the language or its standard library gives, never a project name). Go needs
`GO_BUILTIN=<path of src/builtin/builtin.go>`.
"""
import builtins
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
    return source, names, HAND, set(HAND_BUILTINS["rust"])


def python():
    # Hard keywords only: a soft keyword (match, case, type, _) is a legal identifier.
    version = sys.version.split()[0]
    builtin_names = {n for n in dir(builtins) if not n.startswith("_")}
    return (
        f"le module keyword de CPython {version} : kwlist (sans les mots-clés doux softkwlist)",
        set(keyword.kwlist),
        f"le module builtins de CPython {version} : ses noms publics",
        builtin_names,
    )


def grammar_words(crate, rules, minus_rule=None):
    """The words a tree-sitter grammar reserves: its anonymous alphabetic tokens and the strings of `rules`, without
    the strings of `minus_rule` (the words the grammar still accepts as identifiers)."""
    src = glob.glob(os.path.expanduser(f"~/.cargo/registry/src/*/{crate}/src"))[0]
    with open(f"{src}/node-types.json", encoding="utf-8") as stream:
        nodes = json.load(stream)
    names = {n["type"] for n in nodes if not n["named"] and re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", n["type"])}
    with open(f"{src}/grammar.json", encoding="utf-8") as stream:
        grammar_rules = json.load(stream)["rules"]

    def strings(rule):
        if isinstance(rule, dict):
            if rule.get("type") == "STRING":
                yield rule["value"]
            for value in rule.values():
                yield from strings(value)
        elif isinstance(rule, list):
            for value in rule:
                yield from strings(value)

    for rule in rules:
        names |= set(strings(grammar_rules[rule]))
    if minus_rule:
        names -= set(strings(grammar_rules[minus_rule]))
    names = {n for n in names if re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", n)}
    described = f"la grammaire {crate} : ses jetons alphabétiques anonymes" + (f", {', '.join(rules)}" if rules else "")
    if minus_rule:
        described += f", sans les mots de {minus_rule} (des identifiants valides)"
    return described, names


# The built-in names kept by hand, where the language gives no machine-readable list (the standard library of C, C++,
# C#, Rust, JavaScript). The JavaScript names (`Symbol`, `Map`, `Number`…) belong to JavaScript only.
HAND_BUILTINS = {
    'rust': [
        'Self',
        'Option',
        'Result',
        'Vec',
        'String',
        'Box',
        'Rc',
        'Arc',
        'HashMap',
        'HashSet',
        'BTreeMap',
        'BTreeSet',
        'VecDeque',
        'Cell',
        'RefCell',
        'Mutex',
        'RwLock',
        'Cow',
        'Clone',
        'Copy',
        'Debug',
        'Default',
        'Display',
        'Eq',
        'Hash',
        'Ord',
        'PartialEq',
        'PartialOrd',
        'Send',
        'Sync',
        'Sized',
        'Iterator',
        'IntoIterator',
        'FromIterator',
        'Extend',
        'From',
        'Into',
        'TryFrom',
        'TryInto',
        'AsRef',
        'AsMut',
        'Drop',
        'Deref',
        'DerefMut',
        'Fn',
        'FnMut',
        'FnOnce',
        'println',
        'print',
        'eprintln',
        'eprint',
        'format',
        'panic',
        'assert',
        'vec',
        'dbg',
        'todo',
        'unimplemented',
        'unreachable',
        'bool',
        'char',
        'str',
        'i8',
        'i16',
        'i32',
        'i64',
        'i128',
        'isize',
        'u8',
        'u16',
        'u32',
        'u64',
        'u128',
        'usize',
        'f32',
        'f64',
    ],
    'c': [
        'printf',
        'scanf',
        'malloc',
        'free',
        'calloc',
        'realloc',
        'strlen',
        'strcpy',
        'strcat',
        'strcmp',
        'memcpy',
        'memset',
        'fopen',
        'fclose',
        'fread',
        'fwrite',
        'fprintf',
        'fscanf',
        'exit',
        'abort',
        'atoi',
        'atof',
        'rand',
        'srand',
        'NULL',
        'EOF',
        'stdin',
        'stdout',
        'stderr',
        'size_t',
        'ptrdiff_t',
        'intptr_t',
        'uintptr_t',
        'int8_t',
        'int16_t',
        'int32_t',
        'int64_t',
        'uint8_t',
        'uint16_t',
        'uint32_t',
        'uint64_t',
        'bool',
        'true',
        'false',
    ],
    'cpp': [
        'printf',
        'scanf',
        'malloc',
        'free',
        'calloc',
        'realloc',
        'strlen',
        'strcpy',
        'strcat',
        'strcmp',
        'memcpy',
        'memset',
        'fopen',
        'fclose',
        'fread',
        'fwrite',
        'fprintf',
        'fscanf',
        'exit',
        'abort',
        'atoi',
        'atof',
        'rand',
        'srand',
        'NULL',
        'EOF',
        'stdin',
        'stdout',
        'stderr',
        'size_t',
        'ptrdiff_t',
        'intptr_t',
        'uintptr_t',
        'int8_t',
        'int16_t',
        'int32_t',
        'int64_t',
        'uint8_t',
        'uint16_t',
        'uint32_t',
        'uint64_t',
        'bool',
        'true',
        'false',
        'std',
        'cout',
        'cin',
        'cerr',
        'endl',
        'string',
        'vector',
        'map',
        'set',
        'unordered_map',
        'unordered_set',
        'list',
        'deque',
        'array',
        'pair',
        'tuple',
        'optional',
        'variant',
        'any',
        'shared_ptr',
        'unique_ptr',
        'weak_ptr',
        'make_shared',
        'make_unique',
        'move',
        'forward',
    ],
    'csharp': [
        'object',
        'string',
        'bool',
        'byte',
        'sbyte',
        'char',
        'decimal',
        'double',
        'float',
        'int',
        'uint',
        'long',
        'ulong',
        'short',
        'ushort',
        'nint',
        'nuint',
        'Int32',
        'Int64',
        'Double',
        'Decimal',
        'DateTime',
        'TimeSpan',
        'Guid',
        'Type',
        'Enum',
        'List',
        'Dictionary',
        'HashSet',
        'Queue',
        'Stack',
        'LinkedList',
        'IEnumerable',
        'ICollection',
        'IList',
        'IDictionary',
        'ISet',
        'IQueryable',
        'IOrderedEnumerable',
        'IGrouping',
        'Task',
        'ValueTask',
        'CancellationToken',
        'CancellationTokenSource',
        'IDisposable',
        'IAsyncDisposable',
        'IComparable',
        'IEquatable',
        'ICloneable',
        'Nullable',
        'Console',
        'Debug',
        'Trace',
        'Enumerable',
        'Queryable',
    ],
    'javascript': [
        'Number',
        'String',
        'Boolean',
        'Object',
        'Array',
        'Map',
        'Set',
        'Promise',
        'Date',
        'Error',
        'console',
        'Math',
        'JSON',
        'RegExp',
        'Symbol',
        'isNaN',
    ],
}
HAND = "liste tenue à la main dans data/stop_words/generate.py"

JAVASCRIPT_STOP_WORDS = [
    'if',
    'for',
    'while',
    'return',
    'const',
    'let',
    'var',
    'function',
    'class',
    'extends',
    'implements',
    'import',
    'from',
    'export',
    'default',
    'new',
    'this',
    'super',
    'await',
    'async',
    'switch',
    'case',
    'break',
    'continue',
    'try',
    'catch',
    'finally',
    'throw',
    'true',
    'false',
    'null',
    'undefined',
    'typeof',
    'instanceof',
    'in',
    'of',
]


def c():
    described, names = grammar_words("tree-sitter-c-0.24.2", ["primitive_type", "true", "false", "null"])
    return described, names, HAND, set(HAND_BUILTINS["c"])


def cpp():
    described, names = grammar_words("tree-sitter-cpp-0.23.4", ["primitive_type", "true", "false", "null"])
    return described, names, HAND, set(HAND_BUILTINS["cpp"])


def csharp():
    described, names = grammar_words("tree-sitter-c-sharp-0.23.5", ["boolean_literal", "null_literal"], "_reserved_identifier")
    return described, names, HAND, set(HAND_BUILTINS["csharp"])


def go():
    described, names = grammar_words("tree-sitter-go-0.25.0", ["true", "false", "nil", "iota"])
    path = os.environ["GO_BUILTIN"]
    with open(path, encoding="utf-8") as stream:
        text = stream.read()
    # The predeclared identifiers of builtin.go: its types, functions, constants and variables. The capitalized ones
    # (`Type`, `IntegerType`…) are placeholders of its documentation, not names.
    builtins = set(re.findall(r"^(?:type|func|var|const) ([a-z][A-Za-z0-9_]*)", text, re.M))
    builtins |= set(re.findall(r"^\t?([a-z][A-Za-z0-9_]*) +(?:= |Type)", text, re.M)) & {"true", "false", "iota", "nil"}
    version = re.search(r"go(\d+\.\d+(?:\.\d+)?)", os.environ.get("GO_VERSION", "")) 
    return described, names, f"src/builtin/builtin.go de Go {version.group(1) if version else '?'} : ses noms en minuscules", builtins


def javascript():
    return "la liste historique de JavaScript et TypeScript, gardée telle quelle", set(JAVASCRIPT_STOP_WORDS), HAND, set(HAND_BUILTINS["javascript"])


language = sys.argv[1]
source, names, builtins_source, builtin_names = {"rust": rust, "python": python, "c": c, "cpp": cpp, "csharp": csharp, "go": go, "javascript": javascript}[language]()
print(f"# Les mots vides et les noms intégrés de {language} (codeparsers, `scope_extraction::standard`).")
print("# Généré par data/stop_words/generate.py — ne pas éditer à la main : relancer le générateur.")
print("# Un mot de `names` n'est jamais une référence ; un nom de `builtins` n'est jamais un nom du projet.")
print(f'language = "{language}"')
print(f'source = "{source}"')
print("names = [")
for name in sorted(names):
    print(f'    "{name}",')
print("]")
print(f'builtins_source = "{builtins_source}"')
print("builtins = [")
for name in sorted(builtin_names):
    print(f'    "{name}",')
print("]")
