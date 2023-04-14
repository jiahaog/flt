# !/usr/bin/bash

set -euo pipefail

mode="${1:-}"

pushd example
../third_party/flutter/bin/flutter build bundle
popd

if [ "$mode" = 'debug' ]; then
  export LD_LIBRARY_PATH="$(dirname $(find 'target' -name 'libflutter_engine.so'))"

  cargo build

  rust-lldb target/debug/flt -o run -- \
    --icu-data-path third_party/flutter/bin/cache/artifacts/engine/linux-x64/icudtl.dat \
    --assets-dir example/build/flutter_assets
elif [ "$mode" = 'asan' ]; then
  RUSTFLAGS=-Zsanitizer=address cargo run -Zbuild-std --target x86_64-unknown-linux-gnu -- \
    --icu-data-path third_party/flutter/bin/cache/artifacts/engine/linux-x64/icudtl.dat \
    --assets-dir example/build/flutter_assets
else
  cargo run -- \
    --icu-data-path third_party/flutter/bin/cache/artifacts/engine/linux-x64/icudtl.dat \
    --assets-dir example/build/flutter_assets 
fi
