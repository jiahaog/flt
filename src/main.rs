use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long)]
    assets_dir: String,

    #[arg(long)]
    icu_data_path: String,
}

fn main() {
    let args = Args::parse();

    let embedder = flterminal::Embedder::new(args.assets_dir, args.icu_data_path);
    embedder.wait_for_input();
}
