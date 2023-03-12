//! This build script links the output binary with the Flutter embedder dynamic
//! library.
//!
//! Because building the shared library from source is too complicated, this
//! script simply downloads the headers and prebuilt Flutter dynamic library
//! from the URL specified in `engine.version`, runs `bindgen` to generate Rust
//! bindings from these headers, and then links against the library.
//!
//! The URL within `engine.version` contains a hash, which is the same git
//! commit hash of github.com/flutter/engine where these files are built from.

extern crate bindgen;

use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    // Rerun this script when these files change.
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=engine.version");

    let engine_version = fs::read_to_string("engine.version").unwrap();
    let engine_version = engine_version.trim();

    let out_dir_env = env::var("OUT_DIR").unwrap();

    let out_dir = Path::new(&out_dir_env);

    let embedder_zip_path = out_dir.join("flutter-linux-x64-embedder.zip");

    // Download the zip file containing the Flutter engine dynamic library.
    Command::new("curl")
        .arg(engine_version)
        .arg("--output")
        .arg(embedder_zip_path.clone())
        .output()
        .unwrap();

    // Unzip it.
    Command::new("unzip")
        .arg(embedder_zip_path)
        .arg("-d")
        .arg(out_dir)
        .output()
        .unwrap();

    // There will be two files of interest in the unzipped output:
    // - The headers: flutter_embedder.h
    // - The dynamic library: libflutter_engine.so

    let flutter_embedder_header_path = out_dir.join("flutter_embedder.h");

    // Link against the Flutter dynamic library.
    println!("cargo:rustc-link-search={}", out_dir.to_str().unwrap());
    println!("cargo:rustc-link-lib=flutter_engine");

    // Make `cargo run` work seamlessly.
    //
    // This only works for `cargo run`. The user will have to install the shared
    // library themselves if they use the output binary from `cargo build`.
    println!(
        "cargo:rustc-env=LD_LIBRARY_PATH={}",
        out_dir.to_str().unwrap()
    );

    let bindings = bindgen::Builder::default()
        .header(flutter_embedder_header_path.to_str().unwrap())
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("Unable to generate bindings");

    bindings
        .write_to_file(out_dir.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
