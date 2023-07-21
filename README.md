# flt

`flt` is a **Fl**utter **T**erminal Embedder, implementing the Flutter Engine's [Custom Embedder API](https://github.com/flutter/flutter/wiki/Custom-Flutter-Engine-Embedders).

This embedder draws to the current terminal window using [ANSI Escape Codes](https://en.wikipedia.org/wiki/ANSI_escape_code) to create a [Text-based user interface](https://en.wikipedia.org/wiki/Text-based_user_interface).

https://github.com/jiahaog/flt/assets/7111741/17071ffc-d141-46c2-88c7-e6a6a7f47147

## Why?

Mainly for fun to learn Rust and more about the Flutter Engine. It can also be a quick playground for Flutter without platform-specific or GUI specific dependencies; only a terminal is needed and it can even be used over SSH.

## Supported Platforms / Terminals

This was mainly developed on WSL Linux using the Windows Terminal. It also works on Linux and iTerm2 on macOS. YMMV with other terminal emulators, though it might just work as interfacing with the terminal is done through a cross-platform [library](https://github.com/crossterm-rs/crossterm).

Not working yet:

- Windows
- Terminal.app on macOS

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

### More cli help

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

## Missing Pieces

- [ ] Windows support
- [ ] Fix Terminal.app support on macOS
- [ ] Keyboard support
- [ ] Slow performance
- [ ] Improve semantic label positions

## References

- [Forking Chrome to render in a terminal](https://fathy.fr/carbonyl)
- [brow.sh](https://www.brow.sh/)
