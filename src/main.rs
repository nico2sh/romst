use clap::Clap;

#[derive(Clap)]
#[clap(version = "1.0", author = "Nico Hormazábal")]
struct Opts {
    #[clap(short, long)]
    info: String,
    #[clap(subcommand)]
    subcmd: SubCommand,
}

#[derive(Clap)]
enum SubCommand {
    LoadDat(LoadDat),
}

#[derive(Clap)]
struct LoadDat {
    #[clap(short, long)]
    file: String,
}

fn main() {
    let opts: Opts = Opts::parse();

    match opts.subcmd {
        SubCommand::LoadDat(f) => {
            println!("File to load: {}", f.file)
        }
    }
}
