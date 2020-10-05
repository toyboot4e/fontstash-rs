//! Build script of `fontstash-sys`
//!
//! If the compilation fails, run `cargo clean`.
//!
//! # What it does
//!
//! 1. Pulls `fontstash` if there's not
//! 2. Compiles `fontstash` if it's not found in `OUT_DIR`
//! 4. Links to the output libraries
//! 5. Makes bindings (FFI) to the C libraries
//!
//! # TODOs
//!
//! * TODO: Windows/Linux

use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
};

fn main() {
    self::prepare();
    self::compile();
    self::gen_bindings("fontstash_wrapper.h", "fontstash_bindings.rs");
}

/// Pulls `fontstash`
fn prepare() {
    let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());

    Command::new("git")
        .current_dir(&root)
        .args(&["submodule", "update", "--init", "--recursive"])
        .status()
        .expect("is git in your PATH?");
}

/// Runs `cc` (only when it's necessary) and links the output libraries
fn compile() {
    let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    println!("cargo:rustc-link-lib=fontstash");
    println!("cargo:rerun-if-changed=fontstash_wrapper.h");

    let out_file_path = out_dir.join("libfontstash.a");
    if !out_file_path.is_file() {
        cc::Build::new()
            .file("fontstash.c")
            .define("FONTSTASH_IMPLEMENTATION", Some(""))
            .compile("libfontstash.a");
    }
    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=static=fontstash");
}

/// Generates bindings using a wrapper header file
fn gen_bindings(wrapper: impl AsRef<Path>, dst_file_name: impl AsRef<Path>) {
    let wrapper = wrapper.as_ref();
    let dst_file_name = dst_file_name.as_ref();

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let dst = out_dir.join(&dst_file_name);

    println!("cargo:rerun-if-changed={}", wrapper.display());
    let bindings = bindgen::Builder::default()
        .header(format!("{}", wrapper.display()))
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .unwrap_or_else(|e| {
            panic!(
                "Unable to generate bindings for `{}`. Original error {:?}",
                dst_file_name.display(),
                e
            )
        });

    bindings
        .write_to_file(&dst)
        .unwrap_or_else(|_| panic!("Couldn't write bindings for {}", dst_file_name.display()));
}
