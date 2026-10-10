fn main() {
    let src_dir = std::path::Path::new("src");

    let mut c_config = cc::Build::new();
    // Un drapeau GCC/Clang que `cl` refuse (D8021) : essayé, pas imposé —
    // comme le font tree-sitter-css, -bash et -c-sharp. `-utf-8` pour MSVC,
    // lu sur la cible et non sur l'hôte.
    c_config.std("c11").include(src_dir).flag_if_supported("-Wno-unused-parameter");
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        c_config.flag("-utf-8");
    }

    let parser_path = src_dir.join("parser.c");
    c_config.file(&parser_path);
    println!("cargo:rerun-if-changed={}", parser_path.to_str().unwrap());

    let scanner_path = src_dir.join("scanner.c");
    c_config.file(&scanner_path);
    println!("cargo:rerun-if-changed={}", scanner_path.to_str().unwrap());

    c_config.compile("tree-sitter-scss");
}
