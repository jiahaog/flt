//! This build script links the output binary with the Flutter embedder dynamic
//! library.
//!
//! Because building the shared library from source is too complicated, this
//! script simply downloads the headers and prebuilt Flutter dynamic library,
//! runs `bindgen` to generate Rust bindings from these headers, and then links
//! against the library.
//!
//! The version of the binaries will correspond to the same git commit ref as
//! the same file located in the
//! `third_party/flutter/bin/internal/engine.version` submodule.

extern crate bindgen;

use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

// TODO(jiahaog): Rewrite this into separate scripts for macOS and Linux.
fn main() {
    let engine_ref_path = Path::new("../third_party/flutter/bin/internal/engine.version");

    // Rerun this script when these files change.
    println!("cargo:rerun-if-changed=build.rs");
    println!(
        "cargo:rerun-if-changed={}",
        engine_ref_path.to_str().unwrap()
    );

    let engine_ref = fs::read_to_string(engine_ref_path).unwrap();
    let engine_ref = engine_ref.trim();

    let out_dir_env = env::var("OUT_DIR").unwrap();
    let out_dir = Path::new(&out_dir_env);

    let engine_url = engine_url(engine_ref);
    let downloaded_file = engine_url.split('/').last().unwrap();

    let embedder_zip_path = out_dir.join(downloaded_file);

    // Download the zip file containing the Flutter engine dynamic library.
    assert!(Command::new("curl")
        .arg(engine_url)
        .arg("--output")
        .arg(embedder_zip_path.clone())
        .status()
        .unwrap()
        .success());

    if cfg!(target_os = "macos") {
        let framework_dir = &out_dir.join("FlutterEmbedder.framework");
        unzip(&embedder_zip_path, framework_dir);
        // There's a zip within the zip...
        unzip(
            &framework_dir.join("FlutterEmbedder.framework.zip"),
            framework_dir,
        );
    } else {
        unzip(&embedder_zip_path, out_dir);
    };

    // There will be two files of interest in the unzipped output:
    // (On Linux):
    // - The headers: flutter_embedder.h for bindgen.
    // - The dynamic library: libflutter_engine.so for linking.

    let flutter_embedder_header_path = if cfg!(target_os = "macos") {
        out_dir
            .join("FlutterEmbedder.framework")
            .join("Headers")
            .join("FlutterEmbedder.h")
    } else {
        out_dir.join("flutter_embedder.h")
    };
    let flutter_embedder_header_path = flutter_embedder_header_path.to_str().unwrap();

    let bindings = bindgen::Builder::default()
        .header(flutter_embedder_header_path)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("Unable to generate bindings");

    bindings
        .write_to_file(out_dir.join("bindings.rs"))
        .expect("Couldn't write bindings!");

    // Link against the Flutter shared library.
    if cfg!(target_os = "macos") {
        // On macOS, ld will link using `-l${rustc-link-lib}` which looks for
        // `lib${rustc-link-lib}.dylib.
        //
        // Matches `libFlutterEmbedder.dylib`.
        println!("cargo:rustc-link-lib=framework=FlutterEmbedder");
        println!(
            "cargo:rustc-link-search=framework={}",
            out_dir.to_str().unwrap()
        );
    } else {
        // Matches `libflutter_engine.so`.
        println!("cargo:rustc-link-lib=flutter_engine");
        println!("cargo:rustc-link-search={}", out_dir.to_str().unwrap());
    };

    // Passed to the dependent binary crate to set the runtime search paths.
    println!(
        "cargo:flutter_engine_lib_path={}",
        out_dir.to_str().unwrap()
    );
}

fn engine_url(engine_ref: &str) -> String {
    // This is tricky to figure out and can change between releases.
    //
    // Use the following to find it:
    // ```
    // gsutil ls -r gs://flutter_infra_release/flutter/{engine_ref} | grep embedder
    // ```
    // Source: https://www.industrialflutter.com/blogs/where-to-find-prebuilt-flutter-engine-artifacts/
    if cfg!(target_os = "macos") {
        format!("https://storage.googleapis.com/flutter_infra_release/flutter/{engine_ref}/darwin-x64/FlutterEmbedder.framework.zip")
    } else {
        format!("https://storage.googleapis.com/flutter_infra_release/flutter/{engine_ref}/linux-x64/linux-x64-embedder")
    }
}

fn unzip(src: &Path, dest: &Path) {
    assert!(Command::new("unzip")
        // Overwrite.
        .arg("-o")
        .arg(src)
        .arg("-d")
        .arg(dest)
        .status()
        .unwrap()
        .success());

    // For some reason on macOS, the above command will fail to extract the zip file?
    // And doing it again always works.
    //
    // ```
    // $ "unzip" "-o" "/Users/jiahaog/dev/flt/target/debug/build/flutter-sys-411194cdfb6611b7/out/FlutterEmbedder.framework.zip" "-d" "/Users/jiahaog/dev/flt/target/debug/build/flutter-sys-411194cdfb6611b7/out"
    // Archive:  /Users/jiahaog/dev/flt/target/debug/build/flutter-sys-411194cdfb6611b7/out/FlutterEmbedder.framework.zip
    // inflating: /Users/jiahaog/dev/flt/target/debug/build/flutter-sys-411194cdfb6611b7/out/FlutterEmbedder.framework.zip
    // ```
    // TODO(jiahaog): Figure this out, I suspect antivirus.
    if cfg!(target_os = "macos") {
        assert!(Command::new("unzip")
            // Overwrite.
            .arg("-o")
            .arg(src)
            .arg("-d")
            .arg(dest)
            .status()
            .unwrap()
            .success());
    }
}
