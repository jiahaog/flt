# flutter-sys

Safe bindings for the [Flutter Embedder API](https://github.com/flutter/flutter/wiki/Custom-Flutter-Engine-Embedders).

It only implements the bare minimum required for `flt`.

## Build

Building this package depends on the Flutter Framework checkout in `../third_party/flutter`. It reads from `../third_party/flutter/bin/internal/engine.version` to decide the bindings that should be generated, as well as the shared library to link against.
