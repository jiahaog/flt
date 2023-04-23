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

    // This is tricky to figure out and can change between releases.
    //
    // Use the following to find it:
    // ```
    // gsutil ls -r gs://flutter_infra_release/flutter/{engine_ref} | grep embedder
    // ```
    // Source: https://www.industrialflutter.com/blogs/where-to-find-prebuilt-flutter-engine-artifacts/
    let engine_url = format!("https://storage.googleapis.com/flutter_infra_release/flutter/{engine_ref}/darwin-x64/FlutterEmbedder.framework.zip");

    let out_dir_env = env::var("OUT_DIR").unwrap();

    let out_dir = Path::new(&out_dir_env);

    let embedder_zip_path = out_dir.join(engine_url.split('/').last().unwrap());

    // Download the zip file containing the Flutter engine dynamic library.
    assert!(Command::new("curl")
        .arg(engine_url)
        .arg("--output")
        .arg(embedder_zip_path.clone())
        .status()
        .unwrap()
        .success());

    // Unzip it.
    {
        assert!(Command::new("unzip")
            // Overwrite.
            .arg("-o")
            .arg(embedder_zip_path.clone())
            .arg("-d")
            .arg(out_dir)
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
        assert!(Command::new("unzip")
            // Overwrite.
            .arg("-o")
            .arg(embedder_zip_path.clone())
            .arg("-d")
            .arg(out_dir)
            .status()
            .unwrap()
            .success());
    }

    // ld will link using `-l${rustc-link-lib}` which looks for
    // `lib${rustc-link-lib}.dylib.
    fs::rename(
        out_dir.join("FlutterEmbedder"),
        out_dir.join("libFlutterEmbedder.dylib"),
    )
    .unwrap();

    // There will be two files of interest in the unzipped output:
    // - The headers: flutter_embedder.h
    // - The dynamic library: libflutter_engine.so

    let flutter_embedder_header_path = out_dir.join("Headers").join("FlutterEmbedder.h");

    // Link against the Flutter dynamic library.
    println!("cargo:rustc-link-search={}", out_dir.to_str().unwrap());
    println!("cargo:rustc-link-lib=FlutterEmbedder");

    // Make `cargo run` work seamlessly.
    //
    // This only works for `cargo run`. The user will have to install the shared
    // library themselves if they use the output binary from `cargo build`.
    //
    // On macOS, it seems like it isn't possible to override
    // the dyld library paths with SIP enabled. Need to do something like:
    // ```
    // sudo unzip /Users/jiahaog/dev/flt/target/debug/build/flutter-sys-411194cdfb6611b7/out/FlutterEmbedder.framework.zip -d /Library/Frameworks/FlutterEmbedder.framework
    // ```
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
