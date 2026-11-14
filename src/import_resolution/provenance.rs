//! **La provenance d'un nom importé** (14 novembre 2026, décision de Lucie relayée par l'orchestration) :
//! `builtin` (la bibliothèque standard du langage), `project` (un module du dépôt), `dependency` (une
//! dépendance d'un gestionnaire de paquets) ou `inconnue` — jamais une devinette. codeparsers tranche
//! `builtin` par des listes fixes par langage et `project` par la forme de l'import ; `dependency`
//! demande les manifestes du dépôt (Cargo.lock, package.json…), que lit le consommateur (rag3weaver) :
//! ici, ce qui n'est ni l'un ni l'autre reste `inconnue`.

use crate::import_resolution::c_import_resolver::{CPP_STDLIB_HEADERS, C_STDLIB_HEADERS};
use crate::import_resolution::c_sharp_import_resolver::DOTNET_BCL_NAMESPACES;
use crate::import_resolution::rust_import_resolver::RUST_STD_CRATES;
use crate::import_resolution::type_script_import_resolver::NODE_BUILTINS;

/// La provenance d'un nom importé.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Provenance {
    #[serde(rename = "builtin")]
    Builtin,
    #[serde(rename = "dependency")]
    Dependency,
    #[serde(rename = "project")]
    Project,
    #[serde(rename = "inconnue")]
    Unknown,
}

impl Provenance {
    pub fn as_str(self) -> &'static str {
        match self {
            Provenance::Builtin => "builtin",
            Provenance::Dependency => "dependency",
            Provenance::Project => "project",
            Provenance::Unknown => "inconnue",
        }
    }
}

/// Les modules de la bibliothèque standard de Python : `sys.stdlib_module_names` de **Python 3.14.9**,
/// figé ici (14 novembre 2026). Un module ajouté par une version plus récente sera `inconnue`, pas
/// deviné.
pub const PYTHON_STDLIB_VERSION: &str = "3.14.9";
pub const PYTHON_STDLIB_MODULES: &[&str] = &[
    "__future__", "_abc", "_aix_support", "_android_support", "_apple_support", "_ast",
    "_ast_unparse", "_asyncio", "_bisect", "_blake2", "_bz2", "_codecs", "_codecs_cn", "_codecs_hk",
    "_codecs_iso2022", "_codecs_jp", "_codecs_kr", "_codecs_tw", "_collections", "_collections_abc",
    "_colorize", "_compat_pickle", "_contextvars", "_csv", "_ctypes", "_curses", "_curses_panel",
    "_datetime", "_dbm", "_decimal", "_elementtree", "_frozen_importlib",
    "_frozen_importlib_external", "_functools", "_gdbm", "_hashlib", "_heapq", "_hmac", "_imp",
    "_interpchannels", "_interpqueues", "_interpreters", "_io", "_ios_support", "_json", "_locale",
    "_lsprof", "_lzma", "_markupbase", "_md5", "_multibytecodec", "_multiprocessing", "_opcode",
    "_opcode_metadata", "_operator", "_osx_support", "_overlapped", "_pickle", "_posixshmem",
    "_posixsubprocess", "_py_abc", "_py_warnings", "_pydatetime", "_pydecimal", "_pyio", "_pylong",
    "_pyrepl", "_queue", "_random", "_remote_debugging", "_scproxy", "_sha1", "_sha2", "_sha3",
    "_signal", "_sitebuiltins", "_socket", "_sqlite3", "_sre", "_ssl", "_stat", "_statistics",
    "_string", "_strptime", "_struct", "_suggestions", "_symtable", "_sysconfig", "_thread",
    "_threading_local", "_tkinter", "_tokenize", "_tracemalloc", "_types", "_typing", "_uuid",
    "_warnings", "_weakref", "_weakrefset", "_winapi", "_wmi", "_zoneinfo", "_zstd", "abc",
    "annotationlib", "antigravity", "argparse", "array", "ast", "asyncio", "atexit", "base64",
    "bdb", "binascii", "bisect", "builtins", "bz2", "cProfile", "calendar", "cmath", "cmd", "code",
    "codecs", "codeop", "collections", "colorsys", "compileall", "compression", "concurrent",
    "configparser", "contextlib", "contextvars", "copy", "copyreg", "csv", "ctypes", "curses",
    "dataclasses", "datetime", "dbm", "decimal", "difflib", "dis", "doctest", "email", "encodings",
    "ensurepip", "enum", "errno", "faulthandler", "fcntl", "filecmp", "fileinput", "fnmatch",
    "fractions", "ftplib", "functools", "gc", "genericpath", "getopt", "getpass", "gettext", "glob",
    "graphlib", "grp", "gzip", "hashlib", "heapq", "hmac", "html", "http", "idlelib", "imaplib",
    "importlib", "inspect", "io", "ipaddress", "itertools", "json", "keyword", "linecache",
    "locale", "logging", "lzma", "mailbox", "marshal", "math", "mimetypes", "mmap", "modulefinder",
    "msvcrt", "multiprocessing", "netrc", "nt", "ntpath", "nturl2path", "numbers", "opcode",
    "operator", "optparse", "os", "pathlib", "pdb", "pickle", "pickletools", "pkgutil", "platform",
    "plistlib", "poplib", "posix", "posixpath", "pprint", "profile", "pstats", "pty", "pwd",
    "py_compile", "pyclbr", "pydoc", "pydoc_data", "pyexpat", "queue", "quopri", "random", "re",
    "readline", "reprlib", "resource", "rlcompleter", "runpy", "sched", "secrets", "select",
    "selectors", "shelve", "shlex", "shutil", "signal", "site", "smtplib", "socket", "socketserver",
    "sqlite3", "sre_compile", "sre_constants", "sre_parse", "ssl", "stat", "statistics", "string",
    "stringprep", "struct", "subprocess", "symtable", "sys", "sysconfig", "syslog", "tabnanny",
    "tarfile", "tempfile", "termios", "textwrap", "this", "threading", "time", "timeit", "tkinter",
    "token", "tokenize", "tomllib", "trace", "traceback", "tracemalloc", "tty", "turtle",
    "turtledemo", "types", "typing", "unicodedata", "unittest", "urllib", "uuid", "venv",
    "warnings", "wave", "weakref", "webbrowser", "winreg", "winsound", "wsgiref", "xml", "xmlrpc",
    "zipapp", "zipfile", "zipimport", "zlib", "zoneinfo",
];

/// L'import `source` d'un fichier `fichier` (son extension dit le langage) vient-il de la bibliothèque
/// standard du langage ?
pub fn est_builtin(fichier: &str, source: &str) -> bool {
    let ext = fichier.rsplit('.').next().unwrap_or("");
    let s = source.trim();
    match ext {
        "rs" => s.split("::").next().is_some_and(|r| RUST_STD_CRATES.contains(&r)),
        "js" | "jsx" | "mjs" | "cjs" | "ts" | "tsx" | "mts" | "cts" => {
            let nu = s.strip_prefix("node:");
            let tete = nu.unwrap_or(s).split('/').next().unwrap_or("");
            // `node:x` est toujours le moteur ; sans préfixe, seulement un nom de la liste.
            nu.is_some() || NODE_BUILTINS.contains(&tete)
        }
        "py" | "pyi" => s.split('.').next().is_some_and(|r| PYTHON_STDLIB_MODULES.contains(&r)),
        "c" | "h" => C_STDLIB_HEADERS.contains(&s),
        "cpp" | "cc" | "cxx" | "hpp" | "hh" | "hxx" => CPP_STDLIB_HEADERS.contains(&s) || C_STDLIB_HEADERS.contains(&s),
        // Go : la règle de la chaîne d'outils — un chemin dont le premier élément n'a pas de point
        // est la bibliothèque standard (`net/http`) ; un module tiers commence par un domaine.
        "go" => s.split('/').next().is_some_and(|r| !r.is_empty() && !r.contains('.')),
        "cs" => DOTNET_BCL_NAMESPACES.iter().any(|n| s == *n || s.starts_with(&format!("{n}."))),
        _ => false,
    }
}

/// La provenance tranchée à l'analyse : `project` pour un import local, `builtin` pour la bibliothèque
/// standard, sinon `inconnue` (une dépendance ne se dit qu'avec les manifestes).
pub fn provenance_a_l_analyse(fichier: &str, source: &str, est_local: bool) -> Provenance {
    if est_local {
        Provenance::Project
    } else if est_builtin(fichier, source) {
        Provenance::Builtin
    } else {
        Provenance::Unknown
    }
}
