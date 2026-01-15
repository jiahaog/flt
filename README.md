# flt

`flt` is a **Fl**utter **T**erminal Embedder, implementing the Flutter Engine's [Custom Embedder API](https://docs.flutter.dev/embedded).

With a terminal that [supports](https://sw.kovidgoyal.net/kitty/graphics-protocol/) Kitty graphics, 60fps rendering can be achieved.

![Wonderous app with Kitty Rendering](doc/wonderous_kitty.mov)

Otherwise, it falls back to using [ANSI Escape Codes](https://en.wikipedia.org/wiki/ANSI_escape_code).

![Wonderous app with Kitty Rendering](doc/wonderous_ansi.mov)

This works over SSH though it may be slow depending on the network.

## Supported Platforms / Terminals

Kitty rendering was mostly developed on macOS. Tested on iTerm2 and Ghostty.

ANSI rendering should work on more terminals.

## Checkout

This project uses submodules, so pass the `--recurse-submodules` flag.

```sh
git clone --recurse-submodules git@github.com:jiahaog/flt.git
```

## Usage

Install [Rust](https://www.rust-lang.org/tools/install) first, then at the root of the monorepo, the following command will build the [Sample Flutter App](./sample_app/), and run it with the terminal embedder.

```sh
cargo run
```

### Other Flutter Projects

```sh
cargo run -- <path to the root of your flutter project>
```

### Usage with `flutter run` (Custom Device)

The terminal embedder can be registered as a [Custom Device](https://github.com/flutter/flutter/blob/master/docs/tool/Using-custom-embedders-with-the-Flutter-CLI.md#the-custom-devices-config-file) to use it directly with the `flutter` tool (supporting hot reload, hot restart etc.).

1.  Enable Custom Devices:

    ```sh
    flutter config --enable-custom-devices
    ```

2.  Build the Embedder:

    ```sh
    cargo build --release
    ```

3.  Install Custom Device:

    Run the installation script to configure the custom device and launcher:
    ```sh
    ./install_custom_device.sh
    ```

4.  Run:
    ```sh
    flutter run -d terminal
    ```

### More CLI help for development

```sh
# See help for `flt-cli`.
cargo run -- --help

# See help for `flt`.
cargo run -- --args=--help
```

## Project Structure

- [`flt`](./flt) - The terminal embedder.
- [`flt-cli`](./flt-cli/) - A small CLI utility to make local development easier. By default, the `cargo run` command at the root of the repository will run this.
- [`flutter-sys`](./flutter-sys/) - Safe Rust bindings to the Flutter Embedder API.
- [`sample_app`](./sample_app/) - A sample Flutter Project used for local development.
- [`third_party/flutter`](./third_party/flutter/) - A submodule checkout of the [Flutter Framework](https://github.com/flutter/flutter).

## References

- [Forking Chrome to render in a terminal](https://fathy.fr/carbonyl)
- [brow.sh](https://www.brow.sh/)
