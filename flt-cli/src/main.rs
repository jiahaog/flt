use std::{
    env,
    fs::{self, DirEntry},
    io,
    path::{Path, PathBuf},
    process::Command,
};

use clap::{ArgGroup, Parser, ValueEnum};

/// A CLI for `flt` to make local development easier.
///
/// By default, it builds and runs the example Flutter project, and then builds
/// and starts the `flt` embedder with those Flutter artifacts.
#[derive(Parser, Debug)]
#[command(author, version, about)]
#[clap(group(
            ArgGroup::new("mode")
                // Want --debug to always be the default when not passed. Hence:
                // 1. Set required false here.
                // 2. Set `default_value_t` for `debug` in `Args`.
                // 3. Put the match case for debug at the end, so if --clean is
                //    passed on the command line, debug doesn't take precedence.
                .required(false)
                .args(&["debug", "lldb", "asan", "clean"]),
        ))]
struct Args {
    /// Path to the Flutter project.
    flutter_project_path: Option<String>,

    /// (Default) Build artifacts without optimizations, and then runs the flt
    /// embedder.
    #[clap(long, default_value_t = true)]
    debug: bool,

    /// Run with the lldb debugger attached and primed. Requires rust-lldb.
    #[clap(long)]
    lldb: bool,

    /// Run with AddressSanitizer. Requires nightly Rust.
    #[clap(long)]
    asan: bool,

    /// Removes artifacts from cargo and flutter.
    #[clap(long)]
    clean: bool,

    /// Path to the local engine directory. Only works with --lldb.
    ///
    /// When not passed, defaults to the downloaded prebuilt that will be used
    /// dynamically link `flt`. See ../flutter-sys/build.rs for details.
    #[clap(long)]
    local_engine_out_path: Option<String>,
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
    );

    match (args.debug, args.lldb, args.asan, args.clean) {
        // lldb.
        (_, true, _, _) => {
            assert!(context
                .flutter_tools_command()
                .args(vec!["build", "bundle"])
                .status()
                .unwrap()
                .success());

            assert!(context
                .cargo_command()
                .arg("build")
                .status()
                .unwrap()
                .success());

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
        (_, _, true, _) => {
            assert!(context
                .flutter_tools_command()
                .args(vec!["build", "bundle"])
                .status()
                .unwrap()
                .success());

            assert!(context
                .cargo_command()
                .env("RUSTFLAGS", "-Zsanitizer=address")
                .args(vec!["run", "--package", "flt",])
                .args(vec!["-Zbuild-std", "--target", "x86_64-unknown-linux-gnu",])
                .arg("--")
                .args(context.flt_args())
                .arg("--simple-output")
                .status()
                .unwrap()
                .success());
        }
        // Clean.
        (_, _, _, true) => {
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
        // Debug. This needs to be last - see [Args].
        (true, _, _, _) => {
            assert!(context
                .flutter_tools_command()
                .args(vec!["build", "bundle"])
                .status()
                .unwrap()
                .success());

            assert!(context
                .cargo_command()
                .args(vec!["run", "--package", "flt"])
                .arg("--")
                .args(context.flt_args())
                .status()
                .unwrap()
                .success());
        }
        _ => unreachable!(),
    }
}

struct Context {
    monorepo_root: PathBuf,
    flutter_tools: PathBuf,
    flutter_project_path: PathBuf,
    flutter_project_assets_dir: PathBuf,
    icu_data_path: PathBuf,
    local_engine_out_path: PathBuf,
}

impl Context {
    fn new(
        monorepo_root: PathBuf,
        flutter_project_path: Option<String>,
        local_engine_out_path: Option<String>,
    ) -> Self {
        let flutter_tools = monorepo_root
            .join("third_party")
            .join("flutter")
            .join("bin")
            .join("flutter");

        let flutter_project_path = flutter_project_path
            .map_or(monorepo_root.join("example"), |path_str| {
                Path::new(&path_str).to_path_buf()
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
        vec![
            format!("--icu-data-path={}", self.icu_data_path.to_str().unwrap()),
            format!(
                "--assets-dir={}",
                self.flutter_project_assets_dir.to_str().unwrap()
            ),
        ]
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
