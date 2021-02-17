use clap::{App, Arg, ArgMatches, Clap};
use console::Style;
use env_logger::{Builder, Env, Target};
use romst::{RomsetMode, Romst, sysout::{DatImporterReporterSysOut, ReportReporterSysOut}};
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
    #[clap(about = "Gets information from the database for Romsets")]
    SetInfo(SetInfo),
    #[clap(about = "Shows which sets a Rom is used")]
    RomUsage(RomUsage),
    #[clap(about = "Gets info from the database")]
    DbInfo(DbInfo),
    #[clap(about = "Checks several files or a directory")]
    Check(Check)
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

#[derive(Clap)]
struct Check {
    #[clap(short, long, about = "The ROMST database to use. You can create one with the import command.")]
    db: String,
    #[clap(short, long, about = "A list of files or a directory to review")]
    files: Vec<String>,
    #[clap(short, long, about = "Sets the romset mode, can be either `merge`, `non-merged` or `split`. Default is `non-merged`")]
    set_mode: Option<RomsetMode>,
}

fn create_matches() -> ArgMatches {
    let arg_db = Arg::new("db")
        .about("The ROMST database to use. You can create one with the import command.")
        .long("db")
        .short('d')
        .takes_value(true)
        .required(true);
    let arg_set_mode = Arg::new("set-mode")
        .about("Sets the romset mode, can be either `merge`, `non-merged` or `split`. Default is `non-merged`")
        .long("set-mode")
        .short('m')
        .takes_value(true)
        .required(false);

    let matches = App::new("romst")
        .version("0.1b")
        .author("Nico H. <mail@nico2sh.com>")
        .subcommand(App::new("import")
            .about("Import a DAT file into the database.")
            .arg(Arg::new("file")
                .about("Source DAT file")
                .long("file")
                .short('f')
                .about("Source DAT file.")
                .takes_value(true)
                .required(true))
            .arg(Arg::new("dest")
                .long("db")
                .short('d')
                .about("Destination file. If not specified, uses the source file name as reference.")
                .takes_value(true)
                .required(false))
            .arg(Arg::new("overwrite")
                .short('w')
                .about("Overwrites the destination file if exists.")
                .takes_value(false)
                .required(false)))
        .subcommand(App::new("info")
            .about("Gets information from roms and sets from the database.")
            .subcommand(App::new("data")
                .about("Gets stats info from the database.")
                .arg(arg_db.clone()))
            .subcommand(App::new("set")
                .about("Gets information from the database for Romsets")
                .arg(Arg::new("games")
                    .about("A list of games to retrieve the information from.")
                    .long("games")
                    .short('g')
                    .takes_value(true)
                    .multiple(true)
                    .required(true))
                .arg(arg_db.clone())
                .arg(arg_set_mode.clone()))
            .subcommand(App::new("romusage")
                .about("Shows which sets a Rom is used")
                .arg(Arg::new("game")
                    .about("The game to get the rom to search.")
                    .long("game")
                    .short('g')
                    .takes_value(true)
                    .required(true))
                .arg(Arg::new("rom")
                    .about("The romname to search, if empty, Romst will list all the roms present in the romset.")
                    .long("rom")
                    .short('r')
                    .takes_value(true)
                    .required(false))
                .arg(arg_db.clone())
                .arg(arg_set_mode.clone())))
        .subcommand(App::new("check")
            .about("Checks several files or a directory.")
            .arg(arg_db.clone())
            .arg(arg_set_mode.clone())
            .arg(Arg::new("files")
                .about("A list of files or a directory to check.")
                .short('f')
                .takes_value(true)
                .multiple(true)
                .required(true)))
        .get_matches();

        matches
}

fn main() {
    let mut builder = Builder::from_env(Env::default().default_filter_or("warn"));
    builder.target(Target::Stdout);
    builder.init();

    let matches = create_matches();

    match matches.subcommand() {
        Some(("import", import_matches)) => {
            import(import_matches);
        }
        Some(("info", info_matches)) => {
            info(info_matches);
        }
        Some(_) => {}
        None => {}
    }
    /*let opts: Opts = Opts::parse();

    match opts.subcmd {
        SubCommand::Import(f) => {
            let output = f.destination.unwrap_or(String::from(Path::new(&f.file).with_extension(DB_EXTENSION).to_str().unwrap()));
            let overwrite = f.overwrite;
            let file = f.file;

            let reporter = DatImporterReporterSysOut::new();
            match Romst::import_dat(file.to_owned(), output, overwrite, Some(reporter)) {
                Ok(_) => {}
                Err(e) => { 
                    println!("{} parsing the file {}.\n{}",
                    Style::new().red().apply_to("ERROR"),
                    Style::new().green().apply_to(file),
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
        },
        SubCommand::Check(check) => {
            let db_file = check.db;
            let files = check.files;
            let set_mode = check.set_mode.unwrap_or_default();
            let reporter = ReportReporterSysOut::new();
            match Romst::get_report(db_file, files, set_mode, Some(reporter)) {
                Ok(report) => {
                    println!("{}", report);
                }
                Err(e) => {
                    println!("{} generating a report.\n{}",
                        Style::new().red().apply_to("ERROR"),
                        e);
                }
            }
        }
    }*/
}

fn import(matches: &ArgMatches) {
    let file = matches.value_of("file").unwrap();
    let output = match matches.value_of("dest") {
        Some(o) => {
            o.to_string()
        }
        None => {
            let path = Path::new(&file).with_extension(DB_EXTENSION);
            path.to_str().unwrap().to_string()
        }
    };
    let overwrite = matches.is_present("overwrite");

    let reporter = DatImporterReporterSysOut::new();
    match Romst::import_dat(file, &output, overwrite, Some(reporter)) {
        Ok(_) => {}
        Err(e) => { 
            println!("{} parsing the file {}.\n{}",
            Style::new().red().apply_to("ERROR"),
            Style::new().green().apply_to(file),
            e); 
        }
    }
}

fn info(info_matches: &ArgMatches) {
    match info_matches.subcommand() {
        Some(("data", data_matches)) => info_data(data_matches),
        Some(("set", set_matches)) => set(set_matches),
        Some(("romusage", rom_usage_matches)) => rom_usage(rom_usage_matches),
        Some(_) | None => {}
    }
}

fn info_data(data_matches: &ArgMatches) {
    let db = data_matches.value_of("db").unwrap();

    match Romst::get_db_info(db) {
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

fn set(set_matches: &ArgMatches) {
    let db = set_matches.value_of("db").unwrap();
    let games = set_matches.values_of("games").unwrap().collect::<Vec<_>>();
    let set_mode = match set_matches.value_of("set-mode") {
        Some(mode) => str::parse::<RomsetMode>(mode).unwrap_or_default(),
        None => RomsetMode::default() 
    };

    match Romst::get_set_info(db, games, set_mode) {
        Ok(romsets) => {
            for game_set in romsets {
                println!("{}", game_set);
            }
        }
        Err(e) => { println!("{} getting game info.\n{}",
            Style::new().red().apply_to("ERROR"),
            e); }
    }
}

fn rom_usage(rom_usage_matches: &ArgMatches) {
    let db = rom_usage_matches.value_of("db").unwrap();
    let game = rom_usage_matches.value_of("game").unwrap();
    let rom_name = rom_usage_matches.value_of("rom");
    let set_mode = match rom_usage_matches.value_of("set-mode") {
        Some(mode) => str::parse::<RomsetMode>(mode).unwrap_or_default(),
        None => RomsetMode::default() 
    };

    let execution = match rom_name {
        Some(rom) => {
            Romst::get_rom_usage(db, game, rom, set_mode)
        }
        None => { 
            Romst::get_romset_usage(db, game, set_mode)
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
}