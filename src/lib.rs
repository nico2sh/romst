mod data;
mod error;
mod filesystem;
mod macros;
pub mod sysout;

use console::Style;
use data::{importer::{DatImporter, DatImporterReporter}, models::set::GameSet, reader::{DataReader, RomSearch, sqlite::{DBReader, DBReport}}, reporter::{ReportReporter, Reporter, file_report::Report}, writer::sqlite::DBWriter};
use log::{info, error};
use rusqlite::{Connection, OpenFlags};
use std::{path::Path, str::FromStr};
use serde::{Deserialize, Serialize};
use anyhow::{Result, anyhow};

pub const DEFAULT_WRITE_BUFFER_SIZE: u16 = 5000;

#[derive(Debug, Serialize, Deserialize, Copy, Clone)]
pub enum RomsetMode {
    Merged,
    NonMerged,
    Split,
}

impl Default for RomsetMode {
    fn default() -> Self {
        RomsetMode::NonMerged
    }
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
        if !db_path.exists() {
            return Err(anyhow!("No Database found at `{}`", db_file));
        }
        let conn = Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        Ok(conn)
    }

    pub fn get_data_reader(conn: &Connection) -> Result<DBReader> {
        Ok(DBReader::from_connection(conn))
    }

    pub fn get_data_writer(conn: &mut Connection) -> Result<DBWriter> {
        Ok(DBWriter::from_connection(conn, 500))
    }

    pub fn import_dat<R>(input: String, output_file: String, overwrite: bool, reporter: Option<R>) -> Result<()> where R: DatImporterReporter + 'static {
        println!("Loading file: {}", Style::new().bold().apply_to(&input));
        println!("Output: {}", Style::new().bold().apply_to(&output_file));

        let db_path = Path::new(&output_file);
        if !overwrite && db_path.exists() {
            return Err(anyhow!("Destination file `{}` already exists, choose another output or rename the file.", output_file));
        }

        let mut conn = Romst::get_rw_connection(output_file)?;
        let db_writer = DBWriter::from_connection(&mut conn, DEFAULT_WRITE_BUFFER_SIZE);
        let mut dat_importer = DatImporter::from_path(&input, db_writer)?;
        if let Some(r) = reporter {
            dat_importer.set_reporter(r);
        }

        match dat_importer.load_dat() {
            Ok(_) => info!("Parsing complete"),
            Err(e) => error!("Error parsing file: {}", e)
        };

        Ok(())
    }

    pub fn get_set_info(db_file: String, game_names: Vec<String>, rom_mode: RomsetMode) -> Result<Vec<GameSet>> {
        let mut games =  vec![];
        let conn = Romst::get_r_connection(db_file)?;
        let reader = Romst::get_data_reader(&conn)?;
        for game_name in game_names {
            let roms = reader.get_romset_roms(&game_name, rom_mode)?;
            match reader.get_game(&game_name) {
                Some(game) => {
                    games.push(GameSet::new(game, roms, vec![], vec![]));
                }
                None => {
                    error!("Game {} not found", game_name)
                }
            }
        }

        Ok(games)
    }

    /*pub fn get_rom_usage(db_file: String, game_name: String, rom_name: String, rom_mode: RomsetMode) -> Result<RomSearch> {
        let conn = Romst::get_r_connection(db_file)?;
        let reader = Romst::get_data_reader(&conn)?;
        reader.find_rom_usage(&game_name, &rom_name, rom_mode)
    }

    pub fn get_romset_usage(db_file: String, game_name: String, rom_mode: RomsetMode) -> Result<RomSearch> {
        let conn = Romst::get_r_connection(db_file)?;
        let reader = Romst::get_data_reader(&conn)?;
        reader.get_romset_shared_roms(&game_name, rom_mode)
    }*/

    pub fn get_db_info(db_file: String) -> Result<DBReport> {
        let conn = Romst::get_r_connection(db_file)?;
        let reader = Romst::get_data_reader(&conn)?;
        reader.get_stats()
    }

    pub fn get_report<R>(db_file: String, file_paths: Vec<impl AsRef<Path>>, rom_mode: RomsetMode, reporter: Option<R>) -> Result<Report> where R: ReportReporter + 'static {
        let conn = Romst::get_r_connection(db_file)?;
        let reader = Romst::get_data_reader(&conn)?;

        let mut report = Reporter::new(reader);
        if let Some(reporter) = reporter {
            report.add_reporter(reporter);
        }

        let report = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async { 
                report.check(file_paths, rom_mode).await
             });
        report
    }
}