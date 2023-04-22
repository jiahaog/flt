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
}

fn main() -> Result<(), flt::Error> {
    let args = Args::parse();

    let embedder =
        flt::TerminalEmbedder::new(&args.assets_dir, &args.icu_data_path, args.simple_output)?;
    embedder.wait_for_input()?;

    Ok(())
}
