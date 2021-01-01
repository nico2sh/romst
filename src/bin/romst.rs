use clap::Clap;
use console::Style;
use env_logger::{Builder, Env, Target};
use romst::{RomsetMode, Romst};
use std::path::Path;

const DB_EXTENSION: &str = "rst";

#[derive(Clap)]
#[clap(version = "1.0", author = "Nico Hormazábal")]
struct Opts {
    #[clap(subcommand)]
    subcmd: SubCommand,
}

#[derive(Clap)]
enum SubCommand {
    #[clap(about = "Import a DAT file into the database")]
    Import(ImportDat),
    #[clap(about = "Prints information from a romset")]
    SetInfo(SetInfo),
    #[clap(about = "Shows which sets a Rom is used")]
    RomUsage(RomUsage),
    #[clap(about = "Gets info from the database")]
    DbInfo(DbInfo),
}

#[derive(Clap)]
struct ImportDat {
    #[clap(short, long, about = "Source DAT file.")]
    file: String,
    #[clap(short, long = "dest", about = "Destination file. If not specified, uses the source file name as reference.")]
    destination: Option<String>,
    #[clap(short = 'w', long, takes_value = false, about = "Overwrites the destination file if exists.")]
    overwrite: bool,
}

#[derive(Clap)]
struct SetInfo {
    #[clap(short, long, about = "The ROMST database to use. You can create one with the import command.")]
    db: String,
    #[clap(short, long, about = "A list of games to retrieve the information from")]
    game: Vec<String>,
    #[clap(short, long, about = "Sets the romset mode, can be either `merge`, `non-merged` or `split`. Default is `non-merged`")]
    set_mode: Option<RomsetMode>,
}

#[derive(Clap)]
struct RomUsage {
    #[clap(short, long, about = "The ROMST database to use. You can create one with the import command.")]
    db: String,
    #[clap(short, long, about = "The game to get the rom to search.")]
    game: String,
    #[clap(short, long, about = "The romname to search, if empty, Romst will list all the roms present in the romset.")]
    rom: Option<String>,
    #[clap(short, long, about = "Sets the romset mode, can be either `merge`, `non-merged` or `split`. Default is `non-merged`")]
    set_mode: Option<RomsetMode>,
}

#[derive(Clap)]
struct DbInfo {
    #[clap(short, long, about = "The ROMST database to use. You can create one with the import command.")]
    db: String,
}

fn main() {
    let mut builder = Builder::from_env(Env::default().default_filter_or("warn"));
    builder.target(Target::Stdout);
    builder.init();
    let opts: Opts = Opts::parse();

    match opts.subcmd {
        SubCommand::Import(f) => {
            let output = f.destination.unwrap_or(String::from(Path::new(&f.file).with_extension(DB_EXTENSION).to_str().unwrap()));
            let overwrite = f.overwrite;

            match Romst::import_dat(f.file.to_owned(), output, overwrite) {
                Ok(_) => {}
                Err(e) => { 
                    println!("{} parsing the file {}.\n{}",
                    Style::new().red().apply_to("ERROR"),
                    Style::new().green().apply_to(f.file),
                    e); 
                }
            }
        }
        SubCommand::SetInfo(i) => {
            let db_file = i.db;
            let game_name = i.game;
            let set_mode = i.set_mode;
            match Romst::get_set_info(db_file, game_name.to_owned(), set_mode.unwrap_or_default()) {
                Ok(romsets) => {
                    for game_set in romsets {
                        println!("{}", game_set);
                    }
                }
                Err(e) => { println!("{} getting game info.\n{}",
                    Style::new().red().apply_to("ERROR"),
                    e); }
            }
        },
        SubCommand::RomUsage(ru) => {
            let db_file = ru.db;
            let game_name = ru.game;
            let rom_name = ru.rom;
            let set_mode = ru.set_mode.unwrap_or_default();
            let execution = match rom_name {
                Some(rom) => {
                    Romst::get_rom_usage(db_file, game_name, rom, set_mode)
                }
                None => { 
                    Romst::get_romset_usage(db_file, game_name, set_mode)
                 }
            };

            match execution {
                Ok(result) => {
                    println!("{}", result);
                }
                Err(e) => { println!("{} getting roms info.\n{}",
                    Style::new().red().apply_to("ERROR"),
                    e); }
            }
        },
        SubCommand::DbInfo(db_info) => {
            match Romst::get_db_info(db_info.db) {
                Ok(info) => {
                    println!("{}", info)
                }
                Err(e) => {
                    println!("{} getting roms info.\n{}",
                        Style::new().red().apply_to("ERROR"),
                        e);
                }
            }
        }
    }
}