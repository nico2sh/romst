use clap::{App, Arg, ArgMatches, crate_version};
use anyhow::{Result, anyhow};
use console::Style;
use env_logger::{Builder, Env, Target};
use romst::{RomsetMode, Romst, sysout::{DatImporterReporterSysOut, ReportReporterSysOut}};
use serde::Serialize;
use std::{fmt::Display, path::Path, str::FromStr};

mod ui_cursive;

const DB_EXTENSION: &str = "rst";

enum OutputFormat {
    Json,
    JsonPretty,
    Plain,
}

impl FromStr for OutputFormat {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "json" => Ok(OutputFormat::Json),
            "json-pretty" => Ok(OutputFormat::JsonPretty),
            "plain" => Ok(OutputFormat::Plain),
            _ => Err(anyhow!("Non valid ROM Set Mode, can be either `merged`, `split` or `non-merged`"))
        }
    }
}

impl Default for OutputFormat {
    fn default() -> Self {
        OutputFormat::Json
    }
}

fn create_matches() -> ArgMatches {
    let arg_db = Arg::new("db")
        .about("The ROMST database to use. You can create one with the import command")
        .long("db")
        .short('d')
        .takes_value(true)
        .required(true);
    let arg_set_mode = Arg::new("set-mode")
        .about("Sets the romset mode")
        .long("set-mode")
        .short('m')
        .possible_values(&["merge", "non-merged", "split"])
        .default_value("non-merged")
        .takes_value(true)
        .required(false);
    let arg_format = Arg::new("format")
        .about("Choose the format for the output")
        .long("format")
        .short('f')
        .possible_values(&["json", "json-pretty", "plain"])
        .default_value("json")
        .takes_value(true)
        .required(false);

    let matches = App::new("romst")
        .version(crate_version!())
        .author("Nico H. <mail@nico2sh.com>")
        .subcommand(App::new("ui")
            .about("Loads the UI"))
        .subcommand(App::new("import")
            .about("Import a DAT file into the database")
            .arg(Arg::new("source")
                .about("Source DAT file")
                .long("source")
                .short('s')
                .about("Source DAT file")
                .takes_value(true)
                .required(true))
            .arg(Arg::new("dest")
                .long("db")
                .short('d')
                .about("Destination file. If not specified, uses the source file name as reference")
                .takes_value(true)
                .required(false))
            .arg(Arg::new("overwrite")
                .short('w')
                .about("Overwrites the destination file if exists")
                .takes_value(false)
                .required(false)))
        .subcommand(App::new("info")
            .about("Gets information from roms and sets from the database")
            .subcommand(App::new("data")
                .about("Gets stats info from the database")
                .arg(arg_db.clone())
                .arg(arg_format.clone()))
            .subcommand(App::new("set")
                .about("Gets information from the database for Romsets")
                .arg(Arg::new("games")
                    .about("A list of games to retrieve the information from")
                    .long("games")
                    .short('g')
                    .takes_value(true)
                    .multiple(true)
                    .required(true))
                .arg(arg_db.clone())
                .arg(arg_set_mode.clone())
                .arg(arg_format.clone()))
            .subcommand(App::new("romusage")
                .about("Shows which sets a Rom is used")
                .arg(Arg::new("game")
                    .about("The game to get the rom to search")
                    .long("game")
                    .short('g')
                    .takes_value(true)
                    .required(true))
                .arg(Arg::new("rom")
                    .about("The romname to search, if empty, Romst will list all the roms present in the romset")
                    .long("rom")
                    .short('r')
                    .takes_value(true)
                    .required(false))
                .arg(arg_db.clone())
                .arg(arg_set_mode.clone()))
                .arg(arg_format.clone()))
        .subcommand(App::new("check")
            .about("Checks several files or a directory")
            .arg(Arg::new("source")
                .about("A directory or list of files to check")
                .long("source")
                .short('s')
                .takes_value(true)
                .multiple(true)
                .required(true))
            .arg(arg_db.clone())
            .arg(arg_set_mode.clone())
            .arg(arg_format.clone())
            .arg(Arg::new("report")
                .about("Destination file for the report (if not specified, prints in text format on screen)")
                .long("report")
                .short('r')
                .takes_value(true)
                .required(false)
                .conflicts_with("format")))
        .get_matches();

        matches
}

fn main() {
    let mut builder = Builder::from_env(Env::default().default_filter_or("warn"));
    builder.target(Target::Stdout);
    builder.init();

    let matches = create_matches();

    match matches.subcommand() {
        Some(("ui", ui_matches)) => ui(ui_matches),
        Some(("import", import_matches)) => import(import_matches),
        Some(("info", info_matches)) => info(info_matches),
        Some(("check", check_matches)) => check(check_matches),
        Some(_) => {}
        None => {}
    }
}

fn print_from_format<T: Serialize + Display>(matches: &ArgMatches, obj: T) {
    let format = match matches.value_of("format") {
        Some(f) => str::parse::<OutputFormat>(f).unwrap_or_default(),
        None => OutputFormat::default() 
    };

    match format {
        OutputFormat::Json => {
            let serialized = serde_json::to_string(&obj).unwrap();
            println!("{}", serialized)
        }
        OutputFormat::JsonPretty => {
            let serialized = serde_json::to_string_pretty(&obj).unwrap();
            println!("{}", serialized)
        }
        OutputFormat::Plain => println!("{}", obj)
    };
}

fn ui(_matches: &ArgMatches) {
    match ui_cursive::render() {
        Ok(_) => {}
        Err(e) => {
            println!("{} Loading the UI.\n{}",
                Style::new().red().apply_to("ERROR"), e);
        }
    }
    /*match ui_cursive::render() {
        Ok(_) => {}
        Err(e) => {
            println!("{} Loading the UI.\n{}",
                Style::new().red().apply_to("ERROR"), e);
        }
    }*/
}

fn check(matches: &ArgMatches) {
    let db = matches.value_of("db").unwrap();
    let files = matches.values_of("source").unwrap().collect::<Vec<_>>();
    let set_mode = match matches.value_of("set-mode") {
        Some(mode) => str::parse::<RomsetMode>(mode).unwrap_or_default(),
        None => RomsetMode::default() 
    };

    let reporter = Some(ReportReporterSysOut::new());
    match Romst::get_report(db, files, set_mode, reporter) {
        Ok(report) => {
            if let Some(dest_file) = matches.value_of("report") {
                match Romst::save_report(dest_file, report) {
                    Ok(_) => {
                        println!("{} report saved",
                            Style::new().green().apply_to("SUCCESS"));
                    }
                    Err(e) => {
                        println!("{} saving a report.\n{}",
                            Style::new().red().apply_to("ERROR"), e);
                    }
                }
            } else {
                print_from_format(matches, report);
            }
        }
        Err(e) => {
            println!("{} generating a report.\n{}",
                Style::new().red().apply_to("ERROR"), e);
        }
    }
}

fn import(matches: &ArgMatches) {
    let file = matches.value_of("source").unwrap();
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

fn info(matches: &ArgMatches) {
    match matches.subcommand() {
        Some(("data", data_matches)) => info_data(data_matches),
        Some(("set", set_matches)) => info_set(set_matches),
        Some(("romusage", rom_usage_matches)) => rom_usage(rom_usage_matches),
        Some(_) | None => {}
    }
}

fn info_data(matches: &ArgMatches) {
    let db = matches.value_of("db").unwrap();
    match Romst::get_db_info(db) {
        Ok(info) => {
            print_from_format(matches, info);
        }
        Err(e) => {
            println!("{} getting roms info.\n{}",
                Style::new().red().apply_to("ERROR"),
                e);
        }
    }
}

fn info_set(matches: &ArgMatches) {
    let db = matches.value_of("db").unwrap();
    let games = matches.values_of("games").unwrap().collect::<Vec<_>>();
    let set_mode = match matches.value_of("set-mode") {
        Some(mode) => str::parse::<RomsetMode>(mode).unwrap_or_default(),
        None => RomsetMode::default() 
    };

    match Romst::get_sets_info(db, games, set_mode) {
        Ok(romsets) => {
            print_from_format(matches, romsets);
        }
        Err(e) => { println!("{} getting game info.\n{}",
            Style::new().red().apply_to("ERROR"),
            e); }
    }
}

fn rom_usage(matches: &ArgMatches) {
    let db = matches.value_of("db").unwrap();
    let game = matches.value_of("game").unwrap();
    let rom_name = matches.value_of("rom");
    let set_mode = match matches.value_of("set-mode") {
        Some(mode) => str::parse::<RomsetMode>(mode).unwrap_or_default(),
        None => RomsetMode::default() 
    };

    let execution = match rom_name {
        Some(rom) => {
            Romst::get_rom_usage(db, game, rom, set_mode)
        }
        None => { 
            Romst::get_romset_shared_roms(db, game, set_mode)
            }
    };

    match execution {
        Ok(result) => {
            print_from_format(matches, result);
        }
        Err(e) => { println!("{} getting roms info.\n{}",
            Style::new().red().apply_to("ERROR"),
            e); }
    }
}