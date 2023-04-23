# !/usr/bin/bash

set -euo pipefail

mode="${1:-}"

# Build the app bundle.
pushd example
../third_party/flutter/bin/flutter build bundle
popd

LOCAL_ENGINE_OUT="$HOME/dev/engine/src/out/host_debug_unopt"
FLT_ARGS=(
  '--icu-data-path=third_party/flutter/bin/cache/artifacts/engine/darwin-x64/icudtl.dat'
  '--assets-dir=example/build/flutter_assets'
)

if [ "$mode" = 'debug' ]; then
  # If there is a local engine checkout:
  if [ -d "$LOCAL_ENGINE_OUT" ]; then
    echo "Using libflutter_engine.so in $LOCAL_ENGINE_OUT".
    export LD_LIBRARY_PATH="$LOCAL_ENGINE_OUT"
  else
    # Use the downloaded prebuilt.
    #
    # There may be multiple from different build configurations (hence the
    # `head` command ), but they all should be the same binary.
    export LD_LIBRARY_PATH="$(dirname $(find 'target' -name 'libflutter_engine.so') | head -n 1)"
  fi

  # Sets up rust-lldb so pressing `r` will start the program.
  rust-lldb target/debug/flt -- \
    --simple-output \
    "${FLT_ARGS[@]}"

elif [ "$mode" = 'asan' ]; then
  RUSTFLAGS=-Zsanitizer=address \
    cargo run \
      -Zbuild-std --target x86_64-unknown-linux-gnu \
      -- \
      "${FLT_ARGS[@]}"
else
  cargo run -- "${FLT_ARGS[@]}"
fi
