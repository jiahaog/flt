#!/bin/bash
set -e

CONFIG_DIR="$HOME/.config/flutter"
CONFIG_FILE="$CONFIG_DIR/custom_devices.json"
REPO_ROOT="$(pwd)"

echo "Installing Flutter Terminal Embedder as a custom device..."

# Create config directory
mkdir -p "$CONFIG_DIR"

# Generate custom_devices.json with absolute paths
cat <<EOF > "$CONFIG_FILE"
{
  "custom-devices": [
    {
      "id": "terminal",
      "label": "Terminal",
      "sdkNameAndVersion": "Flutter Terminal Embedder",
      "enabled": true,
      "ping": [
        "echo",
        "pong"
      ],
      "pingSuccessRegex": "pong",
      "install": [
        "cp",
        "-r",
        "\${localPath}",
        "/tmp/\${appName}"
      ],
      "uninstall": [
        "rm",
        "-rf",
        "/tmp/\${appName}"
      ],
      "runDebug": [
        "$REPO_ROOT/launch_flt.sh",
        "$REPO_ROOT/target/release/flt",
        "/tmp/\${appName}",
        "$REPO_ROOT/third_party/flutter/bin/cache/artifacts/engine/darwin-x64/icudtl.dat"
      ],
      "forwardPort": null,
      "forwardPortSuccessRegex": null,
      "screenshot": null
    }
  ]
}
EOF

echo "Success! Custom device configuration written to $CONFIG_FILE"
echo "You can now run: flutter run -d terminal"

