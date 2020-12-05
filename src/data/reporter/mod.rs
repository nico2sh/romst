mod report;

use std::path::{Path, PathBuf};

use crate::{RomsetMode, err, error::RomstIOError, filesystem::{FileReader, FileChecks}};

use self::report::{FileRename, FileReport, Report, SetNameReport, SetReport};

use super::{models::set::GameSet, reader::DataReader};
use anyhow::Result;
use log::{error, warn};


pub struct Reporter<R: DataReader> {
    data_reader: R,
    file_reader: FileReader,
}

impl<R: DataReader> Reporter<R> {
    pub fn new(data_reader: R, file_reader: FileReader) -> Self { Self { data_reader, file_reader } }

    pub fn check(&mut self, file_paths: Vec<impl AsRef<Path>>, rom_mode: &RomsetMode) -> Result<Report> {
        if file_paths.len() == 1 {
            let path = file_paths.get(0).unwrap().as_ref();
            if path.is_dir() {
                return self.check_directory(&path.to_path_buf(), rom_mode)
            }
        }

        self.check_files(file_paths, rom_mode)
    }

    pub fn check_directory(&mut self, file_path: &impl AsRef<Path>, rom_mode: &RomsetMode) -> Result<Report> {
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
            self.check_files(contents, rom_mode)
        } else {
            err!("Path is not a directory")
        }
    }

    pub fn check_files(&mut self, file_paths: Vec<impl AsRef<Path>>, rom_mode: &RomsetMode) -> Result<Report> {
        let game_sets = self.get_sets_from_path(file_paths)?;
        let mut report = Report::new();

        for game_set in game_sets {
            // TODO: fix if the game is not found
            let game = self.data_reader.get_game(&game_set.game.name).unwrap();
            let roms = self.data_reader.get_romset_roms(&game_set.game.name, rom_mode)?;

            let reference_set = GameSet::new(game, roms, vec![], vec![]);

            let set_report = self.compare_set(game_set, reference_set)?;
            report.add_set(FileReport::Set(set_report));
        }

        Ok(report)
    }

    fn get_sets_from_path(&mut self, file_paths: Vec<impl AsRef<Path>>) -> Result<Vec<GameSet>> {
        let mut result = vec![];
        for file_path in file_paths {
            let path = file_path.as_ref();
            if path.is_file() {
                match self.file_reader.get_game_set(&file_path, FileChecks::ALL) {
                    Ok(game_set) => { result.push(game_set) },
                    Err(RomstIOError::NotValidFileError(file_name, _file_type )) => { warn!("File {} is not a valid file", file_name) },
                    Err(e) => { error!("ERROR: {}", e) }
                }
            }
        };

        Ok(result)
    }

    pub fn compare_set(&mut self, game_set: GameSet, reference_set: GameSet) -> Result<SetReport> {
        let mut set_roms = game_set.roms;
        let mut report = SetReport::new(game_set.game.name);

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
                        let file_rename = FileRename::new(set_rom, rom_name);
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

    fn get_db_connection<'a, 'b>(dat_path: &'b impl AsRef<Path>) -> Result<Connection> {
        let mut conn = Connection::open_in_memory_with_flags(OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE)?;
        let writer = DBWriter::from_connection(&mut conn, 100);
        writer.init().unwrap();
        let mut importer = DatImporter::<BufReader<File>, DBWriter>::from_path(dat_path, writer);
        importer.load_dat()?;

        Ok(conn)
    }

    #[test]
    fn get_right_data_from_file() -> Result<()> {
        let path = Path::new("testdata").join("test.dat");
        let conn = get_db_connection(&path)?;
        let data_reader = DBReader::from_connection(&conn);

        let file_reader = FileReader::new();
        let mut reporter = Reporter::new(data_reader, file_reader);

        let game_path = Path::new("testdata").join("split");
        let report = reporter.check(vec![ game_path ], &RomsetMode::Split)?;
        println!("{}", report);

        let report_sets = report.files;
        assert!(report_sets.len() == 5);
        assert!(report_sets.iter().filter(|file| {
            if let FileReport::Set(set_report) = file {
                set_report.name == "game1"
                && set_report.roms_have.len() == 4
                && set_report.roms_missing.len() == 0
                && set_report.roms_to_rename.len() == 0
                && set_report.roms_unneeded.len() == 0
            } else {
                false
            }
        }).collect::<Vec<_>>().len() == 1);
        assert!(report_sets.iter().filter(|file| {
            if let FileReport::Set(set_report) = file {
                set_report.name == "game1a"
                && set_report.roms_have.len() == 2
                && set_report.roms_missing.len() == 0
                && set_report.roms_to_rename.len() == 0
                && set_report.roms_unneeded.len() == 0
            } else {
                false
            }
        }).collect::<Vec<_>>().len() == 1);
        assert!(report_sets.iter().filter(|file| {
            if let FileReport::Set(set_report) = file {
                set_report.name == "game3"
                && set_report.roms_have.len() == 3
                && set_report.roms_missing.len() == 0
                && set_report.roms_to_rename.len() == 0
                && set_report.roms_unneeded.len() == 0
            } else {
                false
            }
        }).collect::<Vec<_>>().len() == 1);
        assert!(report_sets.iter().filter(|file| {
            if let FileReport::Set(set_report) = file {
                set_report.name == "device1"
                && set_report.roms_have.len() == 1
                && set_report.roms_missing.len() == 0
                && set_report.roms_to_rename.len() == 0
                && set_report.roms_unneeded.len() == 0
            } else {
                false
            }
        }).collect::<Vec<_>>().len() == 1);

        Ok(())
    }

    #[test]
    fn get_wrong_data_from_file() -> Result<()> {
        let path = Path::new("testdata").join("test.dat");
        let conn = get_db_connection(&path)?;
        let data_reader = DBReader::from_connection(&conn);

        let file_reader = FileReader::new();
        let mut reporter = Reporter::new(data_reader, file_reader);

        let game_path = Path::new("testdata").join("wrong");
        let report = reporter.check(vec![ game_path ], &RomsetMode::Split)?;
        println!("{}", report);

        let report_sets = report.files;
        assert!(report_sets.len() == 3);
        assert!(report_sets.iter().filter(|file| {
            if let FileReport::Set(set_report) = file {
                set_report.name == "game1"
                && set_report.roms_have.len() == 3
                && set_report.roms_missing.len() == 1
                && set_report.roms_to_rename.len() == 0
                && set_report.roms_unneeded.len() == 0
            } else {
                false
            }
        }).collect::<Vec<_>>().len() == 1);
        assert!(report_sets.iter().filter(|file| {
            if let FileReport::Set(set_report) = file {
                set_report.name == "game2"
                && set_report.roms_have.len() == 2
                && set_report.roms_missing.len() == 0
                && set_report.roms_to_rename.len() == 1
                && set_report.roms_unneeded.len() == 0
            } else {
                false
            }
        }).collect::<Vec<_>>().len() == 1);
        assert!(report_sets.iter().filter(|file| {
            if let FileReport::Set(set_report) = file {
                set_report.name == "game3"
                && set_report.roms_have.len() == 3
                && set_report.roms_missing.len() == 0
                && set_report.roms_to_rename.len() == 0
                && set_report.roms_unneeded.len() == 1
            } else {
                false
            }
        }).collect::<Vec<_>>().len() == 1);

        Ok(())
    }
}