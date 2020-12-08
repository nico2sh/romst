mod report;

use std::path::{Path, PathBuf};
use crate::{RomsetMode, err, error::RomstIOError, filesystem::{FileReader, FileChecks}};
use rayon::prelude::*;
use self::report::{FileRename, FileReport, Report, SetNameReport, SetReport};

use super::{models::{file::DataFile, set::GameSet}, reader::DataReader};
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

    fn check_files(&mut self, file_paths: Vec<impl AsRef<Path>>, rom_mode: &RomsetMode) -> Result<Report> {
        let r: Vec<FileReport> = file_paths.iter().filter_map(|fp| {
            let path = fp.as_ref();
            if path.is_file() {
                match self.file_reader.get_game_set(&path, FileChecks::ALL) {
                    Ok(game_set) => {
                        let file_report = self.on_set_found(game_set, rom_mode).unwrap();
                        Some(file_report)
                    },
                    Err(RomstIOError::NotValidFileError(file_name, _file_type )) => {
                        warn!("File {} is not a valid file", file_name);
                        let file_name = path.to_path_buf().into_os_string().into_string().unwrap_or_else(|ref osstring| {
                            osstring.to_string_lossy().to_string()
                        });
                        // TODO: Unknown file, fix
                        None
                    },
                    Err(e) => {
                        error!("ERROR: {}", e);
                        None
                    }
                }
            } else {
                None
            }
        }).collect();

        let report = Report::from_files(r);

        Ok(report)
    }

    fn on_set_found(&mut self, game_set: GameSet, rom_mode: &RomsetMode) -> Result<FileReport> {
        let mut file_report = FileReport::new(game_set.game.name);

        let rom_usage_result = self.data_reader.get_romsets_from_roms(game_set.roms, rom_mode)?;

        for entry in rom_usage_result {
            let set_name = entry.0;
            let roms = entry.1;

            let set_report = self.compare_roms_with_set(roms.into_iter().collect(), set_name, rom_mode)?;

            file_report.add_set(set_report);
        };

        Ok(file_report)
    }

    pub fn compare_roms_with_set(&mut self, roms: Vec<DataFile>, set_name: String, rom_mode: &RomsetMode) -> Result<SetReport> {
        let mut db_roms = self.data_reader.get_romset_roms(&set_name, rom_mode)?;

        let mut report = SetReport::new(set_name);

        roms.into_iter().for_each(|rom| {
            let found_rom = db_roms.iter().position(|set_rom| {
                rom.deep_compare(&set_rom, FileChecks::ALL, false).ok().unwrap_or_else(|| false)
            });

            let rom_name = rom.name.to_owned().unwrap_or_else(|| {"".to_string()});
            match found_rom {
                Some(set_rom_pos) => {
                    let set_rom = db_roms.remove(set_rom_pos);
                    let set_rom_name = set_rom.name.to_owned().unwrap_or_else(|| {"".to_string()});
                    if rom_name == set_rom_name {
                        report.roms_have.push(set_rom);
                    } else {
                        let file_rename = FileRename::new(rom, set_rom_name);
                        report.roms_to_rename.push(file_rename);
                    }
                }
                None => {
                    report.roms_unneeded.push(rom);
                }
            }
        });

        report.roms_missing = db_roms.into_iter().filter_map(|rom| {
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
        let report = reporter.check(vec![ game_path ], &RomsetMode::Merged)?;

        let report_sets = report.files;
        assert!(report_sets.len() == 5);
        assert!(report_sets.iter().filter(|file_report| {
            file_report.file_name == "game1" &&
            if file_report.sets.len() == 1 {
                let set_report = &file_report.sets[0];
                set_report.name == "game1"
                && set_report.roms_have.len() == 4
                && set_report.roms_missing.len() == 2
                && set_report.roms_to_rename.len() == 0
                && set_report.roms_unneeded.len() == 0
            } else {
                false
            }
        }).collect::<Vec<_>>().len() == 1);
        assert!(report_sets.iter().filter(|file_report| {
            file_report.file_name == "game1a" &&
            if file_report.sets.len() == 1 {
                let set_report = &file_report.sets[0];
                set_report.name == "game1"
                && set_report.roms_have.len() == 2
                && set_report.roms_missing.len() == 4
                && set_report.roms_to_rename.len() == 0
                && set_report.roms_unneeded.len() == 0
            } else {
                false
            }
        }).collect::<Vec<_>>().len() == 1);
        assert!(report_sets.iter().filter(|file_report| {
            file_report.file_name == "device1" &&
            if file_report.sets.len() == 1 {
                let set_report = &file_report.sets[0];
                set_report.name == "device1"
                && set_report.roms_have.len() == 1
                && set_report.roms_missing.len() == 0
                && set_report.roms_to_rename.len() == 0
                && set_report.roms_unneeded.len() == 0
            } else {
                false
            }
        }).collect::<Vec<_>>().len() == 1);
        assert!(report_sets.iter().filter(|file_report| {
            file_report.file_name == "game2" &&
            if file_report.sets.len() == 1 {
                let set_report = &file_report.sets[0];
                set_report.name == "game2"
                && set_report.roms_have.len() == 3
                && set_report.roms_missing.len() == 0
                && set_report.roms_to_rename.len() == 0
                && set_report.roms_unneeded.len() == 0
            } else {
                false
            }
        }).collect::<Vec<_>>().len() == 1);
        assert!(report_sets.iter().filter(|file_report| {
            file_report.file_name == "game3" &&
            if file_report.sets.len() == 1 {
                let set_report = &file_report.sets[0];
                set_report.name == "game3"
                && set_report.roms_have.len() == 3
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
        let report = reporter.check(vec![ &game_path ], &RomsetMode::Merged)?;
        println!("{}", report);

        let report_sets = report.files;
        assert!(report_sets.len() == 4);
        assert!(report_sets.iter().filter(|file_report| {
            file_report.sets.len() == 1
            /*set_report.name == "game1"
            && set_report.roms_have.len() == 3
            && set_report.roms_missing.len() == 1
            && set_report.roms_to_rename.len() == 0
            && set_report.roms_unneeded.len() == 0*/
        }).collect::<Vec<_>>().len() == 1);
        assert!(report_sets.iter().filter(|file_report| {
            file_report.sets.len() == 1
            /*set_report.name == "game2"
            && set_report.roms_have.len() == 2
            && set_report.roms_missing.len() == 0
            && set_report.roms_to_rename.len() == 1
            && set_report.roms_unneeded.len() == 0*/
        }).collect::<Vec<_>>().len() == 1);
        assert!(report_sets.iter().filter(|file_report| {
            file_report.sets.len() == 1
            /*set_report.name == "game3"
            && set_report.roms_have.len() == 3
            && set_report.roms_missing.len() == 0
            && set_report.roms_to_rename.len() == 0
            && set_report.roms_unneeded.len() == 1*/
        }).collect::<Vec<_>>().len() == 1);
        assert!(report_sets.iter().filter(|file_report| {
            file_report.sets.len() == 1
            /*if let FileReport::Unneded(file_name) = file {
                let expected = &game_path.join("info.txt");
                file_name == expected.to_str().unwrap()
            } else {
                false
            }*/
        }).collect::<Vec<_>>().len() == 1);

        Ok(())
    }
}