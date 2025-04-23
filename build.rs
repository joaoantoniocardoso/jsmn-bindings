extern crate cc;

extern crate bindgen;

use std::env;
use std::path::PathBuf;

fn main() {
    // Only regenerate if jsmn changed
    println!("cargo:rerun-if-changed=src/jsmn");

    // Build jsmn library, with optional compiler directives
    let mut build = cc::Build::new();
    let mut builder = bindgen::Builder::default();

    if std::env::var_os("CARGO_FEATURE_PARENT_LINKS").is_some() {
        println!("cargo:rustc-cfg=feature=\"parent-links\"");
        build.define("JSMN_PARENT_LINKS", None);
        builder = builder.clang_arg("-DJSMN_PARENT_LINKS");
    }

    if std::env::var_os("CARGO_FEATURE_STRICT").is_some() {
        println!("cargo:rustc-cfg=feature=\"strict\"");
        build.define("JSMN_STRICT", None);
        builder = builder.clang_arg("-DJSMN_STRICT");
    }

    build
        .file("src/jsmn/jsmn.c")
        .include("src/jsmn")
        .compile("jsmn");

    // Generate bindings for jsmn
    let bindings = builder
        .header("src/jsmn/jsmn.h")
        .allowlist_type("jsmntype_t")
        .allowlist_type("jsmnerr")
        .allowlist_type("jsmntok_t")
        .allowlist_type("jsmn_parser")
        .allowlist_function("jsmn_init")
        .allowlist_function("jsmn_parse")
        .generate()
        .expect("Unable to generate bindings!");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());

    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings.rs!");
}
