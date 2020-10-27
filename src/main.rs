use clap::Clap;
use std::{io::BufReader, fs::File, path::Path};

mod dat;
use dat::DatReader;

mod error;

#[derive(Clap)]
#[clap(version = "1.0", author = "Nico Hormazábal")]
struct Opts {
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
            println!("File to load: {}", f.file);
            let mut dat_reader: DatReader<BufReader<File>> = DatReader::<BufReader<File>>::from_path(Path::new(&f.file));
            match dat_reader.load_dat() {
                Ok(_) => println!("Parsing complete"),
                Err(e) => println!("Error parsing file: {:?}", e)
            }
        }
    }
}
