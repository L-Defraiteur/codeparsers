//! Banc par fichier : mêmes workers que la production, sans résolveur ni service.
use codeparsers::parallel::non_code_project_parser::detect_non_code_parser_type;
use codeparsers::parallel::project_parser::detect_language_from_path;
use codeparsers::parallel::{generic_worker as noncode, parser_worker as code};
use codeparsers::scope_extraction::types::ScopeInfo;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap};
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Instant;

const PARSER_BASE: &str = "bd133850f89d6b12a3ca2b2f6ce238785ed37e3a";
const MAX_FILE_BYTES: u64 = 16 * 1024 * 1024;

#[derive(Deserialize)]
struct Roots {
    schema_version: u32,
    repositories: HashMap<String, PathBuf>,
}
#[derive(Deserialize)]
struct Entry {
    repository_id: String,
    repository_url: String,
    repository_commit: String,
    license_id: String,
    license_path: String,
    relative_path: String,
    sha256: String,
    language: String,
    facade: String,
    stratum: String,
    size_bytes: u64,
}

fn sha256(bytes: &[u8]) -> Result<String, String> {
    // Utilitaire système uniquement ; aucun exécutable provenant des corpus.
    let mut child = Command::new("sha256sum")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    child
        .stdin
        .take()
        .ok_or("sha256sum stdin")?
        .write_all(bytes)
        .map_err(|e| e.to_string())?;
    let output = child.wait_with_output().map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err("sha256sum failed".into());
    }
    let digest = String::from_utf8(output.stdout).map_err(|e| e.to_string())?;
    let digest = digest.split_whitespace().next().ok_or("empty sha256sum")?;
    if !is_hash(digest, 64) {
        return Err("invalid sha256sum output".into());
    }
    Ok(digest.to_owned())
}
fn is_hash(value: &str, size: usize) -> bool {
    value.len() == size
        && value
            .bytes()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
}
fn safe_path(root: &Path, relative: &str) -> Result<PathBuf, String> {
    if relative.is_empty() || relative.contains('\\') || relative.contains('\0') {
        return Err("invalid POSIX relative path".into());
    }
    let mut path = root.to_path_buf();
    for part in Path::new(relative).components() {
        match part {
            Component::Normal(name) => path.push(name),
            _ => return Err("non-relative or non-normal path".into()),
        }
        if fs::symlink_metadata(&path)
            .map_err(|e| e.to_string())?
            .file_type()
            .is_symlink()
        {
            return Err("symlink in corpus path".into());
        }
    }
    if !path.is_file() {
        return Err("not a regular file".into());
    }
    Ok(path)
}
fn peak_rss() -> Option<u64> {
    fs::read_to_string("/proc/self/status")
        .ok()?
        .lines()
        .find_map(|line| {
            line.strip_prefix("VmHWM:")?
                .split_whitespace()
                .next()?
                .parse::<u64>()
                .ok()
        })
        .and_then(|kb| kb.checked_mul(1024))
}
fn scope_counts(
    scopes: &[ScopeInfo],
    counts: &mut BTreeMap<String, usize>,
    references: &mut usize,
    members: &mut usize,
) {
    for scope in scopes {
        *counts.entry(format!("{:?}", scope.r#type)).or_default() += 1;
        *references += scope.identifier_references.len();
        *members += scope.members.as_ref().map_or(0, Vec::len);
        for child in &scope.children {
            scope_counts(
                std::slice::from_ref(child.as_ref()),
                counts,
                references,
                members,
            );
        }
    }
}
fn array_counts(value: &Value, prefix: &str, counts: &mut BTreeMap<String, usize>) {
    if let Some(object) = value.as_object() {
        for (key, value) in object {
            let name = if prefix.is_empty() {
                key.clone()
            } else {
                format!("{prefix}.{key}")
            };
            if let Some(array) = value.as_array() {
                counts.insert(name, array.len());
            } else if value.is_object() {
                array_counts(value, &name, counts);
            }
        }
    }
}
fn code_name(language: &code::SupportedLanguage) -> &'static str {
    use code::SupportedLanguage::*;
    match language {
        Typescript => "typescript",
        Javascript => "javascript",
        Python => "python",
        Rust => "rust",
        Go => "go",
        C => "c",
        Cpp => "cpp",
        Csharp => "csharp",
    }
}
fn noncode_name(parser: &noncode::NonCodeParserType) -> &'static str {
    use noncode::NonCodeParserType::*;
    match parser {
        Markdown => "markdown",
        Css => "css",
        Scss => "scss",
        Html => "html",
        Vue => "vue",
        Svelte => "svelte",
        Generic => "generic",
    }
}
fn analyze(entry: &Entry, content: String) -> Result<Value, String> {
    if entry.facade == "scope" {
        let Some(language) = detect_language_from_path(&entry.relative_path) else {
            return Ok(
                json!({"routing_status":"unsupported_extension","execution_status":"not_attempted"}),
            );
        };
        if code_name(&language) != entry.language {
            return Err("manifest language differs from production routing".into());
        }
        let task = code::ParseFileTask {
            file_path: entry.relative_path.clone(),
            content,
            language,
        };
        let started = Instant::now();
        let result = code::parse_file(&task);
        let elapsed = started.elapsed().as_nanos();
        let mut counts = BTreeMap::new();
        let mut references = 0;
        let mut members = 0;
        scope_counts(&result.scopes, &mut counts, &mut references, &mut members);
        let raw = result.aucun_parseur;
        let captured: usize = counts.values().sum();
        return Ok(
            json!({"routing_status":if raw {"raw_fallback"} else {"supported"},
            "execution_status":"completed", "ast_status":if raw {"not_attempted"} else if result.ast_valid {"valid"} else {"invalid"},
            "ast_issues":result.ast_issues,"elapsed_ns":elapsed,
            "capture_status":if raw {"not_applicable"} else if captured == 0 {"empty"} else {"nonempty"},
            "scopes":if raw {Value::Null} else {json!(captured)},"scope_types":counts,
            "reported_total_scopes":result.total_scopes,"identifier_reference_facts":references,
            "member_facts":members,"import_reference_facts":result.import_references.len(),
            "relations":null,"relations_reason":"project_resolution_not_run"}),
        );
    }
    let Some(parser_type) = detect_non_code_parser_type(&entry.relative_path) else {
        return Ok(
            json!({"routing_status":"unsupported_extension","execution_status":"not_attempted"}),
        );
    };
    if noncode_name(&parser_type) != entry.facade {
        return Err("manifest facade differs from production routing".into());
    }
    let expected_language = match entry
        .relative_path
        .rsplit('.')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "sass" => "sass",
        "mdx" => "mdx",
        _ => noncode_name(&parser_type),
    };
    if entry.language != expected_language {
        return Err("manifest language differs from non-code dialect".into());
    }
    if matches!(
        parser_type,
        noncode::NonCodeParserType::Html | noncode::NonCodeParserType::Svelte
    ) {
        return Ok(
            json!({"routing_status":"unimplemented","execution_status":"not_attempted",
            "diagnostic":if entry.facade == "html" {"production API explicitly returns not implemented"}
                else {"production parser has no language configured; refused before known panic"}}),
        );
    }
    let task = noncode::GenericParseTask {
        file_path: entry.relative_path.clone(),
        content,
        parser_type,
        options: None,
    };
    let started = Instant::now();
    let parsed = noncode::parse_file(&task)?;
    let elapsed = started.elapsed().as_nanos();
    let value = serde_json::to_value(parsed).map_err(|e| e.to_string())?;
    let mut counts = BTreeMap::new();
    array_counts(&value, "", &mut counts);
    let relations = value
        .get("relationships")
        .and_then(Value::as_array)
        .map(Vec::len);
    // Les relations sont une sortie indépendante, pas l'oracle des captures.
    let captured: usize = counts
        .iter()
        .filter(|(name, _)| name.as_str() != "relationships")
        .map(|(_, n)| *n)
        .sum();
    Ok(
        json!({"routing_status":"supported","execution_status":"completed","ast_status":"not_exposed",
        "capture_status":if captured == 0 {"empty"} else {"nonempty"},"elapsed_ns":elapsed,
        "output_array_counts":counts,"scopes":null,"scopes_reason":"specialized_output_not_scope_ir",
        "relations":relations,"relations_kind":"specialized_facts_not_project_resolution"}),
    )
}
fn inspect(entry: &Entry, roots: &Roots) -> Result<Value, String> {
    if !matches!(
        entry.stratum.as_str(),
        "production" | "test" | "documentation"
    ) {
        return Err("unknown stratum".into());
    }
    if !is_hash(&entry.repository_commit, 40) || !is_hash(&entry.sha256, 64) {
        return Err("invalid commit or SHA256".into());
    }
    let repo = entry
        .repository_url
        .strip_prefix("https://")
        .ok_or("repository URL must be canonical HTTPS")?
        .trim_end_matches(".git");
    if !repo.starts_with("github.com/")
        || entry.repository_id != format!("{repo}@{}", entry.repository_commit)
    {
        return Err("repository identity differs from URL/commit".into());
    }
    if entry.license_id.trim().is_empty() {
        return Err("missing license expression".into());
    }
    let root = roots
        .repositories
        .get(&entry.repository_id)
        .ok_or("repository root missing")?;
    if !root.is_absolute() || fs::canonicalize(root).map_err(|e| e.to_string())? != *root {
        return Err("root must be absolute and canonical".into());
    }
    safe_path(root, &entry.license_path)?;
    let path = safe_path(root, &entry.relative_path)?;
    if entry.size_bytes > MAX_FILE_BYTES {
        return Ok(
            json!({"routing_status":"not_checked","execution_status":"not_attempted",
            "diagnostic":"file exceeds published 16MiB bound","exclusion":"size_limit"}),
        );
    }
    let mut bytes = Vec::new();
    File::open(path)
        .map_err(|e| e.to_string())?
        .take(MAX_FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 != entry.size_bytes || sha256(&bytes)? != entry.sha256 {
        return Err("content size/hash mismatch".into());
    }
    let content = match String::from_utf8(bytes) {
        Ok(content) => content,
        Err(error) => {
            return Ok(
                json!({"routing_status":"not_checked","execution_status":"decode_error","diagnostic":error.to_string()}),
            )
        }
    };
    analyze(entry, content)
}
fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args == ["--help"] {
        println!("language_corpus_probe --manifest JSONL --roots JSON --output JSONL\nSerial production workers; 16MiB/file; no resolver. Output must not exist.");
        return Ok(());
    }
    let streaming = args.len() == 3 && args[0] == "--stream" && args[1] == "--roots";
    let (roots_path, manifest_bytes): (&str, Option<Vec<u8>>) = if streaming {
        (&args[2], None)
    } else if args.len() == 6
        && args[0] == "--manifest"
        && args[2] == "--roots"
        && args[4] == "--output"
    {
        (
            &args[3],
            Some(fs::read(&args[1]).map_err(|e| e.to_string())?),
        )
    } else {
        return Err(
            "expected --manifest PATH --roots PATH --output PATH, or --stream --roots PATH".into(),
        );
    };
    let roots_bytes = fs::read(roots_path).map_err(|e| e.to_string())?;
    let roots: Roots = serde_json::from_slice(&roots_bytes).map_err(|e| e.to_string())?;
    if roots.schema_version != 1 {
        return Err("unsupported roots schema".into());
    }
    let mut reader: Box<dyn BufRead + '_> = match &manifest_bytes {
        Some(bytes) => Box::new(BufReader::new(bytes.as_slice())),
        None => Box::new(BufReader::new(std::io::stdin())),
    };
    let mut output: Box<dyn Write> = if streaming {
        Box::new(BufWriter::new(std::io::stdout()))
    } else {
        Box::new(BufWriter::new(
            OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&args[5])
                .map_err(|e| e.to_string())?,
        ))
    };
    let binary_hash = sha256(
        &fs::read(std::env::current_exe().map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?,
    )?;
    let provenance = json!({"parser_base_commit":PARSER_BASE,"binary_sha256":binary_hash,
        "standalone_lock_sha256":sha256(include_bytes!("../Cargo.lock"))?,
        "manifest_sha256":manifest_bytes.as_ref().map(|bytes| sha256(bytes)).transpose()?,"roots_sha256":sha256(&roots_bytes)?,
        "options":"production_worker_defaults; markdown_parse_code_blocks=false",
        "timing":"worker_call_including_lazy_initialization; serial_thread_cache_reused; input_order",
        "peak_rss_source":"/proc/self/status VmHWM KiB converted to bytes; whole process"});
    let mut line = String::new();
    let mut count = 0usize;
    loop {
        line.clear();
        if reader.read_line(&mut line).map_err(|e| e.to_string())? == 0 {
            break;
        }
        count += 1;
        let entry: Entry =
            serde_json::from_str(&line).map_err(|e| format!("manifest line {count}: {e}"))?;
        let identity: Value = serde_json::from_str(&line).map_err(|e| e.to_string())?;
        let result =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| inspect(&entry, &roots)));
        let (measurement, failure) = match result {
            Ok(Ok(value)) => (value, None),
            Ok(Err(error)) => (
                json!({"execution_status":"validation_error","diagnostic":error}),
                Some("validation_error"),
            ),
            Err(error) => {
                let message = error
                    .downcast_ref::<String>()
                    .map(String::as_str)
                    .or_else(|| error.downcast_ref::<&str>().copied())
                    .unwrap_or("non-string panic");
                (
                    json!({"execution_status":"panic","diagnostic":message}),
                    Some("panic"),
                )
            }
        };
        let row = json!({"schema_version":1,"sequence":count,"input":identity,"provenance":provenance,
            "measurement":measurement,"coverage_status":"unjudged","symbols":null,
            "symbols_reason":"no_shared_symbol_oracle_in_file_workers","process_peak_rss_bytes":peak_rss()});
        serde_json::to_writer(&mut output, &row).map_err(|e| e.to_string())?;
        writeln!(output).map_err(|e| e.to_string())?;
        output.flush().map_err(|e| e.to_string())?;
        if let Some(failure) = failure {
            return Err(format!(
                "stopped at line {count}: {failure}; evidence flushed"
            ));
        }
    }
    if count == 0 {
        return Err("empty manifest".into());
    }
    eprintln!("Processed {count} entries; coverage remains unjudged.");
    Ok(())
}
fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
