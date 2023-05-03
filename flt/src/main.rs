use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long)]
    assets_dir: String,

    #[arg(long)]
    icu_data_path: String,

    /// For use when debugging, to disable advanced terminal features so the
    /// console output is not mangled.
    #[arg(long)]
    simple_output: bool,

    /// When enabled, the semantics tree will be dumped to
    /// `/tmp/flt-semantics.txt` whenever it is updated.
    #[arg(long)]
    debug_semantics: bool,

    /// When enabled, semantic labels will be displayed.
    ///
    /// Note that this may be slow.
    #[arg(long)]
    show_semantics: bool,
}

fn main() -> Result<(), flt::Error> {
    let args = Args::parse();

    let mut embedder = flt::TerminalEmbedder::new(
        &args.assets_dir,
        &args.icu_data_path,
        args.simple_output,
        args.debug_semantics,
        args.show_semantics,
    )?;

    embedder.run_event_loop()?;

    Ok(())
}
