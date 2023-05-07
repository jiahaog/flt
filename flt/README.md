# flt

The Terminal Embedder.

## Usage

### `flt-cli`

This will be much easier - see the [documentation](../README.md#usage) at the root of this repository.

### Without `flt-cli`

```sh
cargo build
flt=$(pwd)/target/debug/flt

cd "$FLUTTER_PROJECT_ROOT"
flutter build bundle

# On Linux.
export LD_LIBRARY_PATH="$LIB_FLUTTER_ENGINE_DIR_PATH"

"$flt" \
  --assets-dir="$FLUTTER_PROJECT_ROOT/build/flutter-assets" \
  --icu-data-path="$ICU_DTL_PATH"
```

Flutter projects accepted by the embedder must be built with the Flutter Tool (i.e. the `flutter build` command above) versioned at the same git revision as the [`third_party/flutter`](./third_party/flutter/) submodule, otherwise there will be a "kernel format error".

## Runtime dependencies

### `LIB_FLUTTER_ENGINE_DIR_PATH`

When the Linux dynamic loader starts `flt`, it will search for `libflutter_engine.so` to run the program (see [`man ld.so`](https://man7.org/linux/man-pages/man8/ld.so.8.html)).

This binary can be either compiled from source, or downloaded from Cloud Storage ([documentation](https://github.com/flutter/flutter/wiki/Custom-Flutter-Engine-Embedders)).

Keep in mind that the file paths might be out of date though. To find the most up to date artifact paths, `gsutil` can be used to [list](https://www.industrialflutter.com/blogs/where-to-find-prebuilt-flutter-engine-artifacts/) files in the bucket - we want to look for zip files that mention "embedder" and `libflutter_engine.so` should be zipped inside.

This is also what [`../flutter-sys/build.rs`](../flutter-sys/build.rs) does as part of the build.

#### On Linux

When the file is found, either install it to your systems library paths, or export the directory containing that file as the `LD_LIBRARY_PATH`.

#### On macOS

On macOS, most of the above applies, except that instead of `libflutter_engine.so`, a `FlutterEmbedder.framework` file is needed. See [`../flutter-sys.build.rs`](../flutter-sys/build.rs) for more details.

Note that because there is no equivalent (?) concept of `LD_LIBRARY_PATH` when [SIP](https://developer.apple.com/documentation/security/disabling_and_enabling_system_integrity_protection) is enabled, the [`build.rs`](./build.rs) of this binary sets the `-rpath` linkopt so point to `FlutterEmbedder.framework` that is downloaded as part of the build.

### `ICU_DTL_PATH`

At runtime, the `icudtl.dat` file is needed to launch Flutter. This should be in `../third_party/flutter/bin/cache/artifacts/engine/linux-x64/icudtl.dat` on Linux after you run `flutter build bundle`.

### With `flt-cli`

The `flt-cli`, (i.e. when using `cargo run` from the root of this monorepo) abstracts all of the above away.
