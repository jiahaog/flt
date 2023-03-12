# !/usr/bin/bash

set -euo pipefail

# This will download the Engine binaries, and build our custom Flutter embedder.
cargo build

# This is a hack to replace the Flutter Tester with our binary.
#
# The Flutter Tester is a good candidate for this hack, as it has minimal host
# side dependencies, and can be executed like a real device with
# `flutter run -d flutter-tester` which we do so later in this script.
PATCH=$(cat <<-END
diff --git a/packages/flutter_tools/lib/src/artifacts.dart b/packages/flutter_tools/lib/src/artifacts.dart
index 0d575ebd48..0d9191fcb5 100644
--- a/packages/flutter_tools/lib/src/artifacts.dart
+++ b/packages/flutter_tools/lib/src/artifacts.dart
@@ -654,6 +654,7 @@ class CachedArtifacts implements Artifacts {
           _artifactToFileName(artifact, _platform),
         );
       case Artifact.flutterTester:
+        return '$(pwd)/target/debug/flterminal';
       case Artifact.vmSnapshotData:
       case Artifact.isolateSnapshotData:
       case Artifact.icuData:
END
)

# TODO(jiahaog): Figure out how to make the `git apply` idempotent.
git -C 'third_party/flutter' reset HEAD --hard
echo "$PATCH" | git -C 'third_party/flutter' apply

# If we already have the Flutter Tools compiled, delete it so the patch above
# will apply.
rm -f third_party/flutter/bin/cache/flutter_tools.stamp

# TODO(jiahaog): Make this work for release as well.
FLUTTER_EMBEDDER_PATH=$(find "$(pwd)/target/debug" -name libflutter_engine.so -exec dirname {} \;)

cd example

LD_LIBRARY_PATH="${LD_LIBRARY_PATH:-}:$FLUTTER_EMBEDDER_PATH"  ../third_party/flutter/bin/flutter run -d flutter-tester  --verbose
