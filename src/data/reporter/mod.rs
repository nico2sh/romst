use std::path::Path;

use crate::{RomsetMode, filesystem::{FileReader, FileChecks}};

use super::{models::{report::Report}, reader::DataReader, models::report::FileRename};
use anyhow::Result;


pub struct Reporter<R: DataReader> {
    data_reader: R,
    file_reader: FileReader,
}

impl<R: DataReader> Reporter<R> {
    pub fn new(data_reader: R, file_reader: FileReader) -> Self { Self { data_reader, file_reader } }

    pub fn check_file(&mut self, file_path: &impl AsRef<Path>, rom_mode: &RomsetMode) -> Result<Report> {
        let game_set = self.file_reader.get_game_set(file_path, FileChecks::ALL)?;

        let game = self.data_reader.get_game(&game_set.game.name)?;
        let roms = self.data_reader.get_romset_roms(&game_set.game.name, rom_mode)?;

        let mut set_roms = game_set.roms;

        let mut report = Report::empty(game.name);

        roms.into_iter().for_each(|rom| {
            let found_rom = set_roms.iter().position(|set_rom| {
                rom.deep_compare(&set_rom, FileChecks::ALL, false).ok().unwrap_or_else(|| false)
            });

            let rom_name = rom.name.to_owned().unwrap_or_else(|| {"".to_string()});
            match found_rom {
                Some(set_rom_pos) => {
                    let set_rom = set_roms.remove(set_rom_pos);
                    let set_rom_name = set_rom.name.to_owned().unwrap_or_else(|| {"".to_string()});
                    if rom_name == set_rom_name {
                        report.roms_have.push(set_rom);
                    } else {
                        let file_rename = FileRename { from: set_rom, to: rom_name };
                        report.roms_to_rename.push(file_rename);
                    }
                }
                None => {
                    report.roms_missing.push(rom);
                }
            }
        });

        report.roms_unneeded = set_roms.into_iter().filter_map(|rom| {
            Some(rom)
        }).collect();

        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use std::{io::BufReader, fs::File, path::Path};
    use rusqlite::{Connection, OpenFlags};
    use crate::data::{importer::DatImporter, reader::sqlite::DBReader, writer::{sqlite::DBWriter, DataWriter}};
    use super::*;

    #[test]
    fn get_data_from_file() -> Result<()> {
        let mut conn = Connection::open_in_memory_with_flags(OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE)?;
        let writer = DBWriter::from_connection(&mut conn, 100);
        writer.init().unwrap();
        let path = Path::new("testdata").join("test.dat");
        let mut importer = DatImporter::<BufReader<File>, DBWriter>::from_path(&path, writer);
        importer.load_dat().unwrap();
        let data_reader = DBReader::from_connection(&conn);

        let file_reader = FileReader::new();
        let mut reporter = Reporter::new(data_reader, file_reader);

        let game_path = Path::new("testdata").join("wrong").join("game3.zip");
        let report = reporter.check_file(&game_path, &RomsetMode::Split)?;
        println!("{}", report);
        Ok(())
    }
}