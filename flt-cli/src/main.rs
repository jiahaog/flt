use std::{
    env,
    fs::{self, DirEntry},
    io::{self, stdin},
    path::{Path, PathBuf},
    process::Command,
};

use clap::{ArgGroup, Parser, ValueEnum};
use fs_extra::dir::{self, CopyOptions};

/// A CLI for `flt` to make local development easier.
///
/// It builds the app specified by `flutter_project_path`, and then builds and
/// starts the `flt` embedder using outputs from the previous build.
#[derive(Parser, Debug)]
#[command(author, version, about)]
#[clap(group(
            ArgGroup::new("mode")
                .required(false)
                .args(&["lldb", "asan", "clean"]),
        ))]
struct Args {
    /// Path to the Flutter project.
    ///
    /// Defaults to `../sample_app`. If it is `-`, stdin will be intepreted as a
    /// Dart source file and hosted within a temporary app created from the
    /// `../host_app` template.
    flutter_project_path: Option<String>,

    // TODO(jiahaog): Implement support for Flutter projects in AOT mode.
    /// Build the embedder in release mode, with optimizations.
    ///
    /// For convenience, this will default to if this binary is built in the
    /// release configuration.
    ///
    /// The build mode for the Flutter project will always be "debug mode".
    #[clap(long, default_value_t = cfg!(not(debug_assertions)))]
    release: bool,

    /// Run with the lldb debugger attached and primed. Requires rust-lldb.
    #[clap(long)]
    lldb: bool,

    /// Run with AddressSanitizer. Requires nightly Rust.
    #[clap(long)]
    asan: bool,

    /// Removes artifacts from cargo and flutter.
    #[clap(long)]
    clean: bool,

    /// Whether to run `flutter build bundle`.
    ///
    /// Set this when getting snapshot or invalid argument errors, or when
    /// building a new project.
    #[clap(long, default_value_t = false)]
    flutter_build: bool,

    /// Path to the local engine directory. Only works with --lldb.
    ///
    /// When not passed, defaults to the downloaded prebuilt that will be used
    /// dynamically link `flt`. See ../flutter-sys/build.rs for details.
    #[clap(long)]
    local_engine_out_path: Option<String>,

    // TODO(jiahaog): Find a way to pass this with -- instead.
    /// Arguments that will be passed to `flt`.
    ///
    /// Pass --args=--help to see options.
    #[clap(long)]
    args: Vec<String>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum Mode {
    Debug,
    Lldb,
    Asan,
    Clean,
}

fn main() {
    let args = Args::parse();
    let monorepo_root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();

    let context = Context::new(
        monorepo_root.to_path_buf(),
        args.flutter_project_path,
        args.local_engine_out_path,
        args.args,
    );

    context.check_submodule();

    match (args.lldb, args.asan, args.clean) {
        // lldb.
        (true, _, _) => {
            if args.flutter_build {
                assert!(context
                    .flutter_tools_command()
                    .args(vec!["build", "bundle"])
                    .status()
                    .unwrap()
                    .success());
            }

            let mut cargo_command = context.cargo_command();
            cargo_command.arg("build");
            if args.release {
                cargo_command.arg("--release");
            }
            assert!(cargo_command.status().unwrap().success());

            assert!(Command::new("rust-lldb")
                .current_dir(context.monorepo_root.clone())
                .env("LD_LIBRARY_PATH", context.flutter_engine_path())
                .arg("target/debug/flt")
                .arg("--")
                .args(context.flt_args())
                .arg("--simple-output")
                .status()
                .unwrap()
                .success());
        }
        // Asan.
        (_, true, _) => {
            if args.flutter_build {
                assert!(context
                    .flutter_tools_command()
                    .args(vec!["build", "bundle"])
                    .status()
                    .unwrap()
                    .success());
            }

            let mut cargo_command = context.cargo_command();
            cargo_command
                .env("RUSTFLAGS", "-Zsanitizer=address")
                .args(vec!["run", "--package", "flt"])
                .args(vec!["-Zbuild-std", "--target", "x86_64-unknown-linux-gnu"]);
            if args.release {
                cargo_command.arg("--release");
            }
            assert!(cargo_command
                .arg("--")
                .args(context.flt_args())
                .status()
                .unwrap()
                .success());
        }
        // Clean.
        (_, _, true) => {
            assert!(context
                .flutter_tools_command()
                .arg("clean")
                .status()
                .unwrap()
                .success());

            assert!(context
                .cargo_command()
                .arg("clean")
                .status()
                .unwrap()
                .success());
        }
        // Default. This needs to be last.
        (_, _, _) => {
            if args.flutter_build {
                assert!(context
                    .flutter_tools_command()
                    .args(vec!["build", "bundle"])
                    .status()
                    .unwrap()
                    .success());
            }

            let mut cargo_command = context.cargo_command();
            cargo_command.args(vec!["run", "--package", "flt"]);
            if args.release {
                cargo_command.arg("--release");
            }
            assert!(cargo_command
                .arg("--")
                .args(context.flt_args())
                .status()
                .unwrap()
                .success());
        }
    }
}

struct Context {
    monorepo_root: PathBuf,
    flutter_tools: PathBuf,
    flutter_project_path: PathBuf,
    flutter_project_assets_dir: PathBuf,
    icu_data_path: PathBuf,
    local_engine_out_path: PathBuf,
    flt_args: Vec<String>,
}

impl Context {
    fn new(
        monorepo_root: PathBuf,
        flutter_project_path: Option<String>,
        local_engine_out_path: Option<String>,
        flt_args: Vec<String>,
    ) -> Self {
        let flutter_tools = monorepo_root
            .join("third_party")
            .join("flutter")
            .join("bin")
            .join("flutter");

        let flutter_project_path =
            flutter_project_path.map_or(monorepo_root.join("sample_app"), |path_str| {
                if path_str == "-" {
                    let stdin = stdin();
                    let file = stdin
                        .lines()
                        .map(|line| line.unwrap())
                        .collect::<Vec<String>>()
                        .join("\n");

                    let host_app_template = monorepo_root.join("flt-cli").join("host_app");

                    let host_app = Path::new(&std::env::temp_dir())
                        .join("flt")
                        .join(&std::env::var("USER").unwrap())
                        .join("host_app");
                    if !host_app.exists() {
                        fs::create_dir_all(host_app.clone()).unwrap();

                        dir::copy(
                            host_app_template,
                            host_app.clone().parent().unwrap(),
                            &CopyOptions::new(),
                        )
                        .unwrap();
                    }

                    let main_dart = host_app.join("lib").join("main.dart");
                    fs::write(main_dart, file).unwrap();
                    host_app
                } else {
                    Path::new(&path_str).to_path_buf()
                }
            });

        let flutter_project_assets_dir = flutter_project_path.join("build").join("flutter_assets");

        let icu_data_path = monorepo_root.join(if cfg!(target_os = "macos") {
            "third_party/flutter/bin/cache/artifacts/engine/darwin-x64/icudtl.dat"
        } else {
            "third_party/flutter/bin/cache/artifacts/engine/linux-x64/icudtl.dat"
        });

        let local_engine_out_path = local_engine_out_path.map_or(
            Path::new(&env::var("HOME").unwrap())
                .join("dev")
                .join("engine")
                .join("src")
                .join("out")
                .join("host_debug_unopt"),
            |path_str| Path::new(&path_str).to_path_buf(),
        );

        Self {
            monorepo_root,
            flutter_tools,
            flutter_project_path,
            flutter_project_assets_dir,
            icu_data_path,
            local_engine_out_path,
            flt_args,
        }
    }

    fn flutter_tools_command(&self) -> Command {
        let mut command = Command::new(self.flutter_tools.clone());
        command.current_dir(self.flutter_project_path.clone());
        command
    }

    fn cargo_command(&self) -> Command {
        let mut command = Command::new("cargo");
        command.current_dir(self.monorepo_root.clone());
        command
    }

    fn flt_args(&self) -> Vec<String> {
        let mut result = vec![
            format!("--icu-data-path={}", self.icu_data_path.to_str().unwrap()),
            format!(
                "--assets-dir={}",
                self.flutter_project_assets_dir.to_str().unwrap()
            ),
        ];

        result.extend(self.flt_args.clone());
        result
    }

    fn flutter_engine_path(&self) -> PathBuf {
        if self.local_engine_out_path.is_dir() {
            self.local_engine_out_path.clone()
        } else {
            let libflutter_engine = find_file(&self.monorepo_root.join("target"), &|file| {
                file.file_name()
                    .eq_ignore_ascii_case("libflutter_engine.so")
            })
            .unwrap()
            .unwrap();

            libflutter_engine.path().parent().unwrap().to_path_buf()
        }
    }

    fn check_submodule(&self) {
        let output = Command::new("git")
            .current_dir(self.monorepo_root.clone())
            .arg("submodule")
            .arg("status")
            .output()
            .unwrap();

        for line in String::from_utf8(output.stdout).unwrap().split("\n") {
            if line.starts_with('-') {
                panic!("Submodules in the repository were not initialized. Run the following and try again:\n$ git submodule update --init");
            }
        }
    }
}

fn find_file(dir: &Path, cb: &dyn Fn(&DirEntry) -> bool) -> io::Result<Option<DirEntry>> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                let result = find_file(&path, cb)?;
                if result.is_some() {
                    return Ok(result);
                }
            } else {
                if cb(&entry) {
                    return Ok(Some(entry));
                }
            }
        }
    }
    Ok(None)
}
