use std::path::{Path, PathBuf};

use crate::{RomsetMode, filesystem::{FileReader, FileChecks}};

use super::{models::{report::Report}, models::report::FileRename, reader::DataReader, models::set::GameSet};
use anyhow::Result;


pub struct Reporter<R: DataReader> {
    data_reader: R,
    file_reader: FileReader,
}

impl<R: DataReader> Reporter<R> {
    pub fn new(data_reader: R, file_reader: FileReader) -> Self { Self { data_reader, file_reader } }

    pub fn check_file(&mut self, file_path: Vec<&impl AsRef<Path>>, rom_mode: &RomsetMode) -> Result<Vec<Report>> {
        let game_sets = self.get_sets_from_path(file_path)?;
        let mut reports = vec![];

        for game_set in game_sets {
            // TODO: fix if the game is not found
            let game = self.data_reader.get_game(&game_set.game.name).unwrap();
            let roms = self.data_reader.get_romset_roms(&game_set.game.name, rom_mode)?;

            let reference_set = GameSet::new(game, roms, vec![], vec![]);

            reports.push(self.compare_set(game_set, reference_set)?);
        }

        Ok(reports)
    }

    fn get_sets_from_path(&mut self, file_paths: Vec<impl AsRef<Path>>) -> Result<Vec<GameSet>> {
        let mut result = vec![];
        for file_path in file_paths {
            let path = file_path.as_ref();
            if path.is_dir() {
                let contents = path.read_dir()?.into_iter().filter_map(|dir_entry| {
                    if let Ok(ref entry) = dir_entry {
                        let path = entry.path();
                        Some(path)
                    } else {
                        None
                    }
                }).collect::<Vec<PathBuf>>();
                let mut more = self.get_sets_from_path(contents)?;
                result.append(more.as_mut());
            } else {
                let game_set = self.file_reader.get_game_set(&file_path, FileChecks::ALL)?;
                result.push(game_set);
            }
        };

        Ok(result)
    }

    pub fn compare_set(&mut self, game_set: GameSet, reference_set: GameSet) -> Result<Report> {
        let mut set_roms = game_set.roms;
        let mut report = Report::empty(reference_set.game.name);

        reference_set.roms.into_iter().for_each(|rom| {
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

        let game_path = Path::new("testdata").join("wrong");
        let report = reporter.check_file(vec![&game_path], &RomsetMode::Split)?;
        for set in report {
            println!("{}", set);
        }
        Ok(())
    }
}