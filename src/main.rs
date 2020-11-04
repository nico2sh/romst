mod macros;
mod data;
mod reporter;
mod error;

use data::{writer::DataWriter, dat_reader::DatReader, writer::{sqlite::{DBMode, DBWriter}, sysout::SysOutWriter}};
use clap::Clap;
use env_logger::{Builder, Env, Target};
use log::{info, error};
use std::{io::BufReader, fs::File, path::Path};

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
    let mut builder = Builder::from_env(Env::default().default_filter_or("warn"));
    builder.target(Target::Stdout);
    builder.init();
    // env_logger::init_from_env(Env::default().default_filter_or("warn"));
    let opts: Opts = Opts::parse();

    match opts.subcmd {
        SubCommand::LoadDat(f) => {
            println!("File to load: {}", f.file);
            let db_writer = DBWriter::new(DBMode::File(String::from("test.db"))).unwrap();
            match db_writer.init() {
                Ok(_) => {},
                Err(e) => { error!("Error initializing the database: {}", e) }
            }

            // let mut dat_reader: DatReader<BufReader<File>, SysOutWriter> = DatReader::<BufReader<File>, SysOutWriter>::from_path(Path::new(&f.file), SysOutWriter::new());
            let mut dat_reader: DatReader<BufReader<File>, DBWriter> = DatReader::<BufReader<File>, DBWriter>::from_path(Path::new(&f.file), db_writer);
            match dat_reader.load_dat() {
                Ok(_) => info!("Parsing complete"),
                Err(e) => error!("Error parsing file: {:?}", e)
            }
        }
    }
}
