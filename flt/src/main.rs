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

    /// When the alternate screen is used (default), the Flutter app will be
    /// drawn to a separate buffer, and the currrent terminal buffer will be
    /// restored when this process exits.
    ///
    /// Disabling the alternate screen is helpful for debugging, since
    /// everything from the process will be logged to this alternate buffer
    /// which will be lost.
    #[arg(long)]
    no_alt_screen: bool,

    /// When enabled, logs terminal events.
    ///
    /// Useful for debugging platform-specific / terminal emulator-specific
    /// issues.
    #[arg(long)]
    log_terminal_events: bool,

    /// Disables the kitty graphics protocol even if supported.
    #[arg(long)]
    no_kitty: bool,

    /// Disables GPU rendering (Metal) and forces software rendering.
    #[arg(long)]
    no_gpu: bool,
}

fn main() -> Result<(), flt::Error> {
    let args = Args::parse();

    let mut embedder = flt::TerminalEmbedder::new(
        &args.assets_dir,
        &args.icu_data_path,
        args.simple_output,
        !args.no_alt_screen,
        args.log_terminal_events,
        args.debug_semantics,
        args.no_kitty,
        args.no_gpu,
    )?;

    embedder.run_event_loop()?;

    Ok(())
}
