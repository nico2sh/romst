mod macros;
mod data;
mod reporter;
mod error;
mod filesystem;

use console::Style;
use data::{dat::DatImporter, models::set::GameSet, reader::{sqlite::DBReader, DataReader}, writer::DataWriter, writer::sqlite::DBWriter};
use log::{info, error};
use rusqlite::{Connection, OpenFlags};
use std::{fs::File, io::BufReader, collections::HashMap, path::{Path}, str::FromStr};
use anyhow::{Result, anyhow};

pub const DEFAULT_WRITE_BUFFER_SIZE: u16 = 1000;

pub enum RomsetMode {
    Merged,
    NonMerged,
    Split,
}

impl FromStr for RomsetMode {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "merged" => Ok(RomsetMode::Merged),
            "split" => Ok(RomsetMode::Split),
            "non-merged" => Ok(RomsetMode::NonMerged),
            _ => Err(anyhow!("Non valid ROM Set Mode, can be either `merged`, `split` or `non-merged`"))
        }
    }
}

pub struct Romst {

}

impl Romst {
    fn get_rw_connection(db_file: String) -> Result<Connection>{
        let db_path = Path::new(&db_file);
        let conn = Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE)?;
        Ok(conn)
    }

    fn get_r_connection(db_file: String) -> Result<Connection>{
        let db_path = Path::new(&db_file);
        let conn = Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        Ok(conn)
    }

    pub fn get_data_reader(db_file: String) -> Result<DBReader> {
        let db_path = Path::new(&db_file);
        if !db_path.exists() {
            return Err(anyhow!("No Database found at `{}`", db_file));
        }

        Ok(DBReader::from_connection(Romst::get_r_connection(db_file)?))
    }

    pub fn get_data_writer(db_file: String) -> Result<DBWriter> {
        Ok(DBWriter::from_connection(Romst::get_rw_connection(db_file)?, 500))
    }

    pub fn import_dat(input: String, output: String) -> Result<()> {
        println!("Loading file: {}", Style::new().bold().apply_to(&input));
        println!("Output: {}", Style::new().bold().apply_to(&output));

        let db_path = Path::new(&output);
        if db_path.exists() {
            return Err(anyhow!("Destination file `{}` already exists, choose another output or rename the file.", output));
        }

        let db_writer = DBWriter::new(db_path, DEFAULT_WRITE_BUFFER_SIZE)?;
        match db_writer.init() {
            Ok(_) => {},
            Err(e) => { error!("Error initializing the database: {}", e) }
        }
        let mut dat_reader: DatImporter<BufReader<File>, DBWriter> = DatImporter::<BufReader<File>, DBWriter>::from_path(Path::new(&input), db_writer);

        //let mut dat_reader: DatReader<BufReader<File>, SysOutWriter> = DatReader::<BufReader<File>, SysOutWriter>::from_path(Path::new(&f.file), SysOutWriter::new());
        match dat_reader.load_dat() {
            Ok(_) => info!("Parsing complete"),
            Err(e) => error!("Error parsing file: {}", e)
        };

        Ok(())
    }

    pub fn get_set_info(db_file: String, game_names: Vec<String>, rom_mode: RomsetMode) -> Result<Vec<GameSet>> {
        let mut games =  vec![];
        let reader = Romst::get_data_reader(db_file)?;
        for game_name in game_names {
            let roms = reader.get_romset_roms(&game_name, &rom_mode)?;
            games.push(GameSet::new(reader.get_game(&game_name)?, roms, vec![], vec![]));
        }

        Ok(games)
    }

    pub fn get_rom_usage(db_file: String, game_name: String, rom_name: String) -> Result<HashMap<String, Vec<String>>> {
        let reader = Romst::get_data_reader(db_file)?;
        reader.find_rom_usage(&game_name, &rom_name)
    }

    pub fn get_romset_usage(db_file: String, game_name: String) -> Result<HashMap<String, Vec<String>>> {
        let reader = Romst::get_data_reader(db_file)?;
        reader.get_romset_shared_roms(&game_name)
    }
}