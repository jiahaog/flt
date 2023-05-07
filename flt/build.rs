fn main() {
    let flutter_engine_lib_path = std::env::var(
        // `DEP_${Cargo.toml.links of flutter-sys crate}_${actual envar pair in build.rs}`.
        "DEP_FLUTTER_ENGINE_FLUTTER_ENGINE_LIB_PATH",
    )
    .unwrap();

    // TODO(jiahaog): Figure out a better bundling solution that isn't coupled to
    // `cargo run`.
    if cfg!(target_os = "macos") {
        // On macOS, with SIP enabled there's no way to override the framework paths
        // with an environment variable at runtime. So make the binary look in the
        // cargo output directory for the `FlutterEmbedder.framework`.
        println!("cargo:rustc-link-arg=-Wl,-rpath,{flutter_engine_lib_path}");
    } else {
        // On Linux, it is sufficient to set this environment variable to find the
        // `libflutter_engine.so` at runtime.
        //
        // Note that this is an environment set at **runtime** when only when running
        // with `cargo run`.
        println!("cargo:rustc-env=LD_LIBRARY_PATH={flutter_engine_lib_path}");
    }
}
