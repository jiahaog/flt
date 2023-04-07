# !/usr/bin/bash

set -euo pipefail

pushd example
../third_party/flutter/bin/flutter build bundle
popd

cargo run -- \
  --icu-data-path third_party/flutter/bin/cache/artifacts/engine/linux-x64/icudtl.dat \
  --assets-dir example/build/flutter_assets \
