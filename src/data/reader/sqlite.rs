use std::{collections::HashSet, iter::FromIterator, collections::HashMap};

use anyhow::Result;
use log::{error, warn};
use rusqlite::{Connection, ToSql, params};

use crate::{RomsetMode, data::models::{file::{DataFile, FileType::Rom}, game::Game}};

use super::DataReader;

#[derive(Debug)]
pub struct DBReader<'d> {
    conn: &'d Connection,
}

impl <'d> DBReader <'d>{
    pub fn from_connection(conn: &'d Connection) -> Self {
        Self { conn }
    }

    fn get_merged_set_roms(&self, game_name: &String) -> Result<Vec<DataFile>> {
        let mut roms_stmt = self.conn.prepare("SELECT DISTINCT game_roms.name, roms.sha1, roms.md5, roms.crc, roms.size, roms.status
            FROM game_roms 
            JOIN roms ON roms.id = game_roms.rom_id WHERE (game_roms.game_name = ?1 OR game_roms.parent = ?1);")?;
        let roms_rows = roms_stmt.query_map(params![ game_name ], |row| {
            Ok(DataFile {
                file_type: Rom,
                name: row.get(0)?,
                sha1: row.get(1)?,
                md5: row.get(2)?,
                crc: row.get(3)?,
                size: row.get(4)?,
                status: row.get(5)?,
            })
        })?.filter_map(|row| row.ok());

        let roms: HashSet<DataFile> = Vec::from_iter(roms_rows).drain(..).collect();
        Ok(Vec::from_iter(roms))
    }

    fn get_nonmerged_set_roms(&self, game_name: &String) -> Result<Vec<DataFile>> {
        let mut nonmerged_stmt = self.conn.prepare("SELECT game_roms.name, roms.sha1, roms.md5, roms.crc, roms.size, roms.status
            FROM game_roms 
            JOIN roms ON roms.id = game_roms.rom_id
            WHERE game_roms.game_name = ?1")?;
        let roms_rows = nonmerged_stmt.query_map(params![ game_name ], |row| {
            Ok(DataFile {
                file_type: Rom,
                name: row.get(0)?,
                sha1: row.get(1)?,
                md5: row.get(2)?,
                crc: row.get(3)?,
                size: row.get(4)?,
                status: row.get(5)?,
            })
        })?.filter_map(|row| row.ok());

        Ok(Vec::from_iter(roms_rows))
    }

    fn get_split_set_roms(&self, game_name: &String) -> Result<Vec<DataFile>> {
        let mut roms_stmt = self.conn.prepare("SELECT DISTINCT game_roms.name, roms.sha1, roms.md5, roms.crc, roms.size, roms.status
            FROM game_roms 
            JOIN roms ON roms.id = game_roms.rom_id WHERE (game_roms.game_name = ?1 AND game_roms.parent IS NULL);")?;
        let roms_rows = roms_stmt.query_map(params![ game_name ], |row| {
            Ok(DataFile {
                file_type: Rom,
                name: row.get(0)?,
                sha1: row.get(1)?,
                md5: row.get(2)?,
                crc: row.get(3)?,
                size: row.get(4)?,
                status: row.get(5)?,
            })
        })?.filter_map(|row| row.ok());

        let roms: HashSet<DataFile> = Vec::from_iter(roms_rows).drain(..).collect();
        Ok(Vec::from_iter(roms))
    }

    pub fn get_stats(&self) -> Result<String> {
        let mut stmt = self.conn.prepare("SELECT COUNT(*) FROM games;")?;
        let result: u32 = stmt.query_row(params![], |row| {
            Ok(row.get(0)?)
        })?;

        return Ok(format!("{} games", result))
    }

    fn get_rom_ids_from_files(&self, roms: Vec<DataFile>) -> Result<(Vec<u32>, Vec<DataFile>)> {
        let mut result = vec![];
        let mut not_found = vec![];
        for rom in roms {
            let mut params: Vec<&dyn ToSql> = vec![];
            let mut statement_where = vec![];
            let mut param_num = 1;
            if rom.sha1.is_some() {
                params.push(rom.sha1.as_ref().unwrap());
                statement_where.push(format!("(sha1 = ?{} OR sha1 IS NULL)", param_num));
                param_num = param_num + 1;
            }
            
            if rom.md5.is_some() {
                params.push(rom.md5.as_ref().unwrap());
                statement_where.push(format!("(md5 = ?{} OR md5 IS NULL)", param_num));
                param_num = param_num + 1;
            }

            if rom.crc.is_some() {
                params.push(rom.crc.as_ref().unwrap());
                statement_where.push(format!("(crc = ?{} OR crc IS NULL)", param_num));
                param_num = param_num + 1;
            }

            if rom.size.is_some() {
                params.push(rom.size.as_ref().unwrap());
                statement_where.push(format!("(size = ?{} OR size IS NULL)", param_num));
            }

            let statement = "SELECT id FROM roms WHERE ".to_string() +
                &statement_where.join(" AND ") + ";";

            let mut rom_stmt = self.conn.prepare_cached(&statement)?;
            let rom_result: rusqlite::Result<u32> = rom_stmt.query_row(params, |row| {
                Ok(row.get(0)?)
            });

            match rom_result {
                Err(rusqlite::Error::QueryReturnedNoRows) => {
                    not_found.push(rom);
                    warn!("No ROM found");
                },
                Ok(id) => {
                    result.push(id);
                },
                Err(e) => error!("Error adding a rom: {}", e),
            };
        }
        
        Ok((result, not_found))
    }
}

impl <'d> DataReader for DBReader<'d> {
    fn get_game(&self, game_name: &String) -> Option<Game> {
        let mut game_stmt = self.conn.prepare("SELECT name, clone_of, rom_of, source_file, info_desc, info_year, info_manuf
            FROM games WHERE name = ?1;").ok()?;
        let game_result= game_stmt.query_row(params![ game_name ], |row| {
            Ok(
                Game {
                    name: row.get(0)?,
                    clone_of: row.get(1)?,
                    rom_of: row.get(2)?,
                    source_file: row.get(3)?,
                    info_description: row.get(4)?,
                    info_year: row.get(5)?,
                    info_manufacturer: row.get(6)?
                }
            )
        });

        match game_result {
            Ok(game) => {
                Some(game)
            },
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                None
            },
            Err(e) => {
                error!("Unexpected error reading the roms database: {}", e);
                None
            }
        }
    }

    fn get_romset_roms(&self, game_name: &String, rom_mode: &RomsetMode) -> Result<Vec<DataFile>> {
        let result = match rom_mode {
            RomsetMode::NonMerged => {
                self.get_nonmerged_set_roms(&game_name)
            },
            RomsetMode::Merged => {
                self.get_merged_set_roms(&game_name)
            },
            RomsetMode::Split => {
                self.get_split_set_roms(&game_name)
            }
        };

        result
    }

    fn find_rom_usage(&self, game_name: &String, rom_name: &String) -> Result<HashMap<String, Vec<String>>> {
        let mut roms_stmt = self.conn.prepare("SELECT DISTINCT game_roms.game_name, game_roms.name as romname
            FROM game_roms 
                WHERE game_roms.rom_id = (SELECT game_roms.rom_id FROM game_roms 
                WHERE game_roms.game_name = ?1 AND game_roms.name = ?2) ORDER BY game_roms.game_name;
        ")?;
        let roms_rows = roms_stmt.query_map(params![ game_name, rom_name ], |row| {
            Ok((row.get(0)?,
                row.get(1)?))
        })?.filter_map(|row| row.ok());

        let mut result: HashMap<String, Vec<String>> = HashMap::new();
        for item in roms_rows {
            let game_name = item.0;
            let rom_name = item.1;

            result.entry(game_name).or_insert(vec![]).push(rom_name);
        }

        Ok(result)
    }

    fn get_romset_shared_roms(&self, game_name: &String) -> Result<HashMap<String, Vec<String>>> {
        let mut roms_stmt = self.conn.prepare("SELECT DISTINCT game_roms.game_name, game_roms.name as romname
            FROM game_roms WHERE game_roms.rom_id IN (SELECT game_roms.rom_id FROM game_roms 
                WHERE game_roms.game_name = ?1) ORDER BY game_roms.game_name;
        ")?;
        let roms_rows = roms_stmt.query_map(params![ game_name ], |row| {
            Ok((row.get(0)?,
                row.get(1)?))
        })?.filter_map(|row| row.ok());

        let mut result: HashMap<String, Vec<String>> = HashMap::new();
        for item in roms_rows {
            let game_name = item.0;
            let rom_name = item.1;

            result.entry(game_name).or_insert(vec![]).push(rom_name);
        }

        Ok(result)
    }

    fn get_romsets_from_roms(&self, roms: Vec<DataFile>, rom_mode: &RomsetMode) {
        
    }
}

#[cfg(test)]
mod tests {
    use std::{io::BufReader, fs::File, path::Path};
    use rusqlite::{Connection, OpenFlags};
    use crate::data::{importer::DatImporter, reader::sqlite::DBReader, models::file::FileType, writer::{sqlite::DBWriter, DataWriter}};
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
    fn find_rom_id_from_sha1() -> Result<()> {
        let path = Path::new("testdata").join("test.dat");
        let conn = get_db_connection(&path)?;
        let data_reader = DBReader::from_connection(&conn);

        let mut roms = vec![];
        let mut rom1 = DataFile::new(FileType::Rom);
        rom1.sha1 = Some("8bb3a81b9fa2de5163f0ffc634a998c455bcca25".to_string());
        roms.push(rom1);
        let result = data_reader.get_rom_ids_from_files(roms)?;
        let rom_ids = result.0;
        let not_found = result.1;

        assert!(rom_ids.len() == 1);
        assert!(rom_ids[0] == 1);
        assert!(not_found.len() == 0);

        Ok(())
    }

    #[test]
    fn find_rom_id_from_sha1_and_crc() -> Result<()> {
        let path = Path::new("testdata").join("test.dat");
        let conn = get_db_connection(&path)?;
        let data_reader = DBReader::from_connection(&conn);

        let mut roms = vec![];
        let mut rom1 = DataFile::new(FileType::Rom);
        rom1.sha1 = Some("802e076afc412be12db3cb8c79523f65d612a6cf".to_string());
        rom1.crc = Some("dc20b010".to_string());
        roms.push(rom1);
        let result = data_reader.get_rom_ids_from_files(roms)?;
        let rom_ids = result.0;
        let not_found = result.1;

        assert!(rom_ids.len() == 1);
        assert!(rom_ids[0] == 0);
        assert!(not_found.len() == 0);

        Ok(())
    }

    #[test]
    fn find_rom_id_from_sha1_and_crc_and_md5_but_no_md5_in_db() -> Result<()> {
        let path = Path::new("testdata").join("test.dat");
        let conn = get_db_connection(&path)?;
        let data_reader = DBReader::from_connection(&conn);

        let mut roms = vec![];
        let mut rom1 = DataFile::new(FileType::Rom);
        rom1.sha1 = Some("8273bfebe84dd41a5d237add8f9d03ac9bb0ef54".to_string());
        rom1.crc = Some("1b736d41".to_string());
        rom1.md5 = Some("0de4e413deb3ae71e9326d70df4d1a27".to_string());
        roms.push(rom1);
        let result = data_reader.get_rom_ids_from_files(roms)?;
        let rom_ids = result.0;
        let not_found = result.1;

        assert!(rom_ids.len() == 1);
        assert!(rom_ids[0] == 4);
        assert!(not_found.len() == 0);

        Ok(())
    }

    #[test]
    fn dont_find_rom_id_from_sha1_and_crc_and_wrong_size() -> Result<()> {
        let path = Path::new("testdata").join("test.dat");
        let conn = get_db_connection(&path)?;
        let data_reader = DBReader::from_connection(&conn);

        let mut roms = vec![];
        let mut rom1 = DataFile::new(FileType::Rom);
        rom1.sha1 = Some("8273bfebe84dd41a5d237add8f9d03ac9bb0ef54".to_string());
        rom1.crc = Some("1b736d41".to_string());
        rom1.size = Some(1024);
        roms.push(rom1);
        let result = data_reader.get_rom_ids_from_files(roms)?;
        let rom_ids = result.0;
        let not_found = result.1;

        assert!(rom_ids.len() == 0);
        assert!(not_found.len() == 1);

        Ok(())
    }

}