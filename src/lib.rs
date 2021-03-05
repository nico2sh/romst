pub mod data;
mod error;
mod filesystem;
mod macros;
pub mod sysout;

use console::Style;
use data::{importer::{DatImporter, DatImporterReporter}, models::set::GameSet, reader::{DataReader, RomSearch, SetDependencies, sqlite::{DBReader, DBReport}}, reporter::{ReportReporter, Reporter, scan_report::ScanReport}, writer::sqlite::DBWriter};
use log::{info, error};
use rusqlite::{Connection, OpenFlags};
use std::{fmt::Display, fs::File, io::Write, path::Path, str::FromStr};
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

impl Display for RomsetMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RomsetMode::Merged => {
                write!(f, "Merged")
            }
            RomsetMode::NonMerged => {
                write!(f, "Non Merged")
            }
            RomsetMode::Split => {
                write!(f, "Split")
            }
        }
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

#[derive(Debug, Serialize, Deserialize)]
pub struct GameSetsInfo {
    pub game_sets: Vec<GameSet>
}

impl GameSetsInfo {
    pub fn new(game_sets: Vec<GameSet>) -> Self { Self { game_sets } }
}


impl Display for GameSetsInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for game_set in &self.game_sets {
            writeln!(f, "{}", game_set)?;
        };
        Ok(())
    }
}

impl Romst {
    fn get_rw_connection<S>(db_file: S) -> Result<Connection> where S: AsRef<str>{
        let db_path = Path::new(db_file.as_ref());
        let conn = Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE)?;
        Ok(conn)
    }

    fn get_r_connection<S>(db_file: S) -> Result<Connection> where S: AsRef<str> {
        let db_path = Path::new(db_file.as_ref());
        if !db_path.exists() {
            return Err(anyhow!("No Database found at `{}`", db_file.as_ref()));
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

    pub fn import_dat<R, S>(input: S, output_file: S, overwrite: bool, reporter: Option<R>) -> Result<()> where R: DatImporterReporter + 'static, S: AsRef<str> {
        println!("Loading file: {}", Style::new().bold().apply_to(input.as_ref()));
        println!("Output: {}", Style::new().bold().apply_to(output_file.as_ref()));

        let db_path = Path::new(output_file.as_ref());
        if !overwrite && db_path.exists() {
            return Err(anyhow!("Destination file `{}` already exists, choose another output or rename the file.", output_file.as_ref()));
        }

        let mut conn = Romst::get_rw_connection(output_file)?;
        let db_writer = DBWriter::from_connection(&mut conn, DEFAULT_WRITE_BUFFER_SIZE);
        let mut dat_importer = DatImporter::from_path(&input.as_ref().to_string(), db_writer)?;
        if let Some(r) = reporter {
            dat_importer.set_reporter(r);
        }

        match dat_importer.load_dat() {
            Ok(_) => info!("Parsing complete"),
            Err(e) => error!("Error parsing file: {}", e)
        };

        Ok(())
    }

    pub fn get_set_info<S>(db_file: S, game_names: Vec<S>, rom_mode: RomsetMode) -> Result<GameSetsInfo> where S: AsRef<str> {
        let mut games =  vec![];
        let conn = Romst::get_r_connection(db_file)?;
        let reader = Romst::get_data_reader(&conn)?;
        for game_name in game_names {
            let roms = reader.get_romset_roms(game_name.as_ref(), rom_mode)?.1.into_iter().map(|db_rom| {
                db_rom.file
            }).collect();
            let device_refs = reader.get_devices_for_game(game_name.as_ref())?;
            match reader.get_game(game_name.as_ref()) {
                Some(game) => {
                    games.push(GameSet::new(game, roms, vec![], vec![], device_refs.dependencies));
                }
                None => {
                    error!("Game {} not found", game_name.as_ref())
                }
            }
        }

        Ok(GameSetsInfo::new(games))
    }

    pub fn get_rom_usage<S>(db_file: S, game_name: S, rom_name: S, rom_mode: RomsetMode) -> Result<RomSearch> where S: AsRef<str> {
        let conn = Romst::get_r_connection(db_file)?;
        let reader = Romst::get_data_reader(&conn)?;
        reader.find_rom_usage(game_name.as_ref(), rom_name.as_ref(), rom_mode)
    }

    pub fn get_romset_usage<S>(db_file: S, game_name: S, rom_mode: RomsetMode) -> Result<RomSearch> where S: AsRef<str> {
        let conn = Romst::get_r_connection(db_file)?;
        let reader = Romst::get_data_reader(&conn)?;
        reader.get_romset_shared_roms(game_name.as_ref(), rom_mode)
    }

    pub fn get_romset_dependencies<S>(db_file: S, game_name: S, rom_mode: RomsetMode) -> Result<SetDependencies> where S: AsRef<str> {
        let conn = Romst::get_r_connection(db_file)?;
        let reader = Romst::get_data_reader(&conn)?;
        let mut result = reader.get_devices_for_game(game_name.as_ref())?;

        // If we are in split mode, we add the parent as a dependency
        match rom_mode {
            RomsetMode::Split => {
                if let Some(game) = reader.get_game(game_name.as_ref()) {
                    if let Some(clone_of) = game.clone_of {
                        result.dependencies.push(clone_of);
                    }
                }
            }
            _ => {}
        }

        Ok(result)
    }

    pub fn get_db_info<S>(db_file: S) -> Result<DBReport> where S: AsRef<str>{
        let conn = Romst::get_r_connection(db_file)?;
        let reader = Romst::get_data_reader(&conn)?;
        reader.get_stats()
    }

    pub fn get_report<R, S>(db_file: S, file_paths: Vec<impl AsRef<Path>>, rom_mode: RomsetMode, progress_reporter: Option<R>) -> Result<ScanReport> where R: ReportReporter + 'static, S: AsRef<str> {
        let conn = Romst::get_r_connection(db_file)?;
        let reader = Romst::get_data_reader(&conn)?;

        let mut reporter = Reporter::new(reader);
        if let Some(progress_reporter) = progress_reporter {
            reporter.add_reporter(progress_reporter);
        }

        let report = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async { 
                reporter.check(file_paths, rom_mode).await
             });
        report
    }

    pub fn save_report<S>(output_file: S, report: ScanReport) -> Result<()> where S: AsRef<str> {
        let encoded: Vec<u8> = bincode::serialize(&report)?;
        let mut file = File::create(output_file.as_ref())?;
        file.write_all(&encoded)?;

        Ok(())
    }

}