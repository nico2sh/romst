use std::{collections::HashSet, fmt::Display, iter::FromIterator};

use anyhow::Result;
use console::Style;
use log::{debug, error, warn};
use rusqlite::{Connection, ToSql, params};

use crate::{RomsetMode, data::models::{file::{DataFile, DataFileInfo, FileType::{self, Rom}}, game::Game}};

use super::{DataReader, FileCheckSearch, RomSearch};

#[derive(Debug)]
pub struct SearchRomIds {
    pub found: Vec<(u32, DataFile)>,
    pub not_found: Vec<DataFile>,
    pub ignored: Vec<DataFile>
}

impl SearchRomIds {
    fn new() -> Self { Self { found: vec![], not_found: vec![], ignored: vec![] } }

    fn add_found(&mut self, id: u32, file: DataFile) {
        self.found.push((id, file));
    }

    fn add_not_found(&mut self, rom: DataFile) {
        self.not_found.push(rom);
    }
}

pub struct DBReport {
    pub games: u32,
    pub roms: u32,
    pub roms_in_games: u32,
    pub samples: u32,
    pub device_refs: u32,
}

impl DBReport {
    pub fn new() -> Self { Self { games: 0, roms: 0, roms_in_games: 0, samples: 0, device_refs: 0 } }
}

impl Display for DBReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", Style::new().bold().yellow().apply_to("Database info"))?;
        writeln!(f, "- Games: {}", self.games)?;
        writeln!(f, "- Roms: {}", self.roms)?;
        writeln!(f, "- Roms in Games: {}", self.roms_in_games)?;
        writeln!(f, "- Samples: {}", self.samples)?;
        writeln!(f, "- Device References: {}", self.device_refs)
    }
}

/*
Game name = row.get(0)?;
Rom name = row.get(1)?;
Rom sha1 = row.get(2)?;
Rom md5 = row.get(3)?;
Rom crc = row.get(4)?;
Rom size = row.get(5)?;
Rom status = row.get(6)?;
Rom parent = row.get(7)?;
Game clone_of = row.get(8)?;
Rom id = row.get(9)?;
*/
const ROMS_QUERY: &str = "SELECT DISTINCT game_roms.game_name, game_roms.name as rom_name, roms.sha1, roms.md5, roms.crc, roms.size, game_roms.status, game_roms.parent, games.clone_of, roms.id
                FROM game_roms JOIN roms ON game_roms.rom_id = roms.id JOIN games ON game_roms.game_name = games.name";

#[derive(Debug)]
pub struct DBReader<'d> {
    conn: &'d Connection,
}

impl <'d> DBReader <'d>{
    pub fn from_connection(conn: &'d Connection) -> Self {
        Self { conn }
    }

    pub fn get_stats(&self) -> Result<DBReport> {
        let mut db_report = DBReport::new();

        let mut stmt = self.conn.prepare("SELECT COUNT(*) FROM games;")?;
        let games: u32 = stmt.query_row(params![], |row| {
            Ok(row.get(0)?)
        })?;
        db_report.games = games;

        let mut stmt = self.conn.prepare("SELECT COUNT(*) FROM roms;")?;
        let roms: u32 = stmt.query_row(params![], |row| {
            Ok(row.get(0)?)
        })?;
        db_report.roms = roms;

        let mut stmt = self.conn.prepare("SELECT COUNT(*) FROM game_roms;")?;
        let roms_in_games: u32 = stmt.query_row(params![], |row| {
            Ok(row.get(0)?)
        })?;
        db_report.roms_in_games = roms_in_games;

        let mut stmt = self.conn.prepare("SELECT COUNT(*) FROM samples;")?;
        let samples: u32 = stmt.query_row(params![], |row| {
            Ok(row.get(0)?)
        })?;
        db_report.samples = samples;

        let mut stmt = self.conn.prepare("SELECT COUNT(*) FROM devices;")?;
        let device_refs: u32 = stmt.query_row(params![], |row| {
            Ok(row.get(0)?)
        })?;
        db_report.device_refs = device_refs;

        return Ok(db_report);
    }

    fn find_sets_for_roms(&self, rom_ids: Vec<(u32, DataFile)>, rom_mode: RomsetMode) -> Result<RomSearch> {
        let mut params: Vec<&dyn ToSql> = vec![];
        let mut ids_cond = String::new();
        let mut i = 0;
        rom_ids.iter().for_each(|rom_id| {
            if i != 0 {
                ids_cond.push_str(", ");
            }
            i += 1;                     
            ids_cond.push_str("?");
            ids_cond.push_str(&i.to_string());
            params.push(&rom_id.0);
        });

        // We do a query with all the roms we received, the result will return all sets associateed with these roms
        let query = ROMS_QUERY.to_string() + " WHERE game_roms.rom_id IN (" + &ids_cond + ") ORDER BY game_roms.game_name;";

        let mut roms_stmt = self.conn.prepare(&query)?;
        let roms_rows = roms_stmt.query_map(params, |row| {
            let mut data_file_info = DataFileInfo::new(FileType::Rom);
            let rom_id: u32 = row.get(9)?;
            let name = match rom_ids.as_slice().into_iter().find(|p| { p.0 == rom_id }) {
                Some(result) => {
                    let data_file = &result.1;
                   & data_file.name
                }
                None => {
                    ""
                }
            };

            data_file_info.sha1 = row.get(2)?;
            data_file_info.md5 = row.get(3)?;
            data_file_info.crc = row.get(4)?;
            data_file_info.size = row.get(5)?;
            let data_file = DataFile::new(name, data_file_info);
            Ok((row.get(0)?,
                data_file,
                row.get(7)?,
                row.get(8)?))
        })?.filter_map(|row| row.ok());

        let mut result = RomSearch::new();
        for item in roms_rows {
            let game_name = item.0;
            let rom = item.1;
            let game_parent: Option<String> = item.2;
            let clone_of: Option<String> = item.3;

            match rom_mode {
                RomsetMode::Merged => {
                    if let Some(game_parent_name) = clone_of {
                        result.add_file_for_set(game_parent_name, rom);
                    } else {
                        result.add_file_for_set(game_name, rom);
                    }
                }
                RomsetMode::NonMerged => {
                    result.add_file_for_set(game_name, rom);
                }
                RomsetMode::Split => {
                    if game_parent == None {
                        result.add_file_for_set(game_name, rom);
                    }
                }
            }
        }

        Ok(result)
    }

    pub fn get_ids_from_files(conn: &Connection, files: Vec<DataFile>) -> Result<SearchRomIds> {
        let mut result = SearchRomIds::new();
        for rom_file in files {
            let rom = &rom_file.info;

            match &rom_file.status {
                Some(status) if status.to_lowercase() == "nodump" => {
                    // We ignore the ones without dump
                    result.ignored.push(rom_file);
                },
                _ => {
                    let mut params: Vec<(&str, &dyn ToSql)> = vec![];
                    let mut statement_where = vec![];
                    let mut has_hash = false;

                    if let Some(ref sha1) = rom.sha1 {
                        has_hash = true;
                        params.push((":sha1", sha1));
                        statement_where.push("(sha1 = :sha1 OR sha1 IS NULL)");
                    }
                    if let Some(ref md5) = rom.md5 {
                        has_hash = true;
                        params.push((":md5", md5));
                        statement_where.push("(md5 = :md5 OR md5 IS NULL)");
                    }

                    if !has_hash {
                        warn!("Rom `{}` has no hash value, it could match any other rom, should be ignored", rom_file);
                        result.not_found.push(rom_file);
                    } else {
                        if let Some(ref crc) = rom.crc {
                            params.push((":crc", crc));
                            statement_where.push("(crc = :crc OR crc IS NULL)");
                        }
                        if let Some(ref size) = rom.size {
                            params.push((":size", size));
                            statement_where.push("(size = :size OR size IS NULL)");
                        }

                        // Minimum fields to find, has to have at least md5 or sha1
                        statement_where.push("(sha1 IS NOT NULL OR md5 IS NOT NULL)");

                        let statement = "SELECT id FROM roms WHERE ".to_string() +
                            &statement_where.join(" AND ") + ";";
                        
                        let mut rom_stmt = conn.prepare_cached(&statement)?;
                        let query_rom_result: Vec<u32> = rom_stmt.query_map_named(params.as_slice(), |row| {
                            Ok(row.get(0)?)
                        })?.filter_map(|row| row.ok() ).collect();

                        match query_rom_result.len() {
                            0 => {
                                debug!("No ROM found in DB: {}", rom);
                                result.add_not_found(rom_file);
                            },
                            1 => {
                                debug!("Found ROM in DB: {}", rom);
                                result.add_found(query_rom_result[0], rom_file);
                            },
                            n => {
                                // TODO: There is a corner case which is, if the search has a sha1, and the DB has a md5 it may match as both with match against the null value
                                warn!("Found more than one rom ({}) on the query, ROM: {}", n, rom_file);
                                result.ignored.push(rom_file);
                            }
                        }
                    }
                }
            }
        }
        
        Ok(result)
    }
}

impl <'d> DataReader for DBReader<'d> {
    fn get_game<S>(&self, game_name: S) -> Option<Game> where S: AsRef<str> + rusqlite::ToSql {
        let mut game_stmt = self.conn.prepare("SELECT name, clone_of, rom_of, source_file, sample_of, info_desc, info_year, info_manuf
            FROM games WHERE name = ?1;").ok()?;
        let game_result= game_stmt.query_row(params![ game_name ], |row| {
            Ok(
                Game {
                    name: row.get(0)?,
                    clone_of: row.get(1)?,
                    rom_of: row.get(2)?,
                    source_file: row.get(3)?,
                    sample_of: row.get(4)?,
                    info_description: row.get(5)?,
                    info_year: row.get(6)?,
                    info_manufacturer: row.get(7)?
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

    fn get_romset_roms<S>(&self, game_name: S, rom_mode: RomsetMode) -> Result<Vec<DataFile>> where S: AsRef<str> + rusqlite::ToSql {
        let mut query = ROMS_QUERY.to_string();
        match rom_mode {
            RomsetMode::Merged => {
                query.push_str(" WHERE (game_roms.game_name = ?1 OR games.clone_of = ?1);");
            }
            RomsetMode::NonMerged => {
                query.push_str(" WHERE game_roms.game_name = ?1");
            }
            RomsetMode::Split => {
                query.push_str(" WHERE (game_roms.game_name = ?1 AND game_roms.parent IS NULL);");
            }
        }

        let mut roms_stmt = self.conn.prepare(&query)?;
        let roms_rows = roms_stmt.query_map(params![ game_name ], |row| {
            Ok(DataFile {
                name: row.get(1)?,
                info: DataFileInfo {
                    file_type: Rom,
                    sha1: row.get(2)?,
                    md5: row.get(3)?,
                    crc: row.get(4)?,
                    size: row.get(5)?,
                },
                status: row.get(6)?
            })
        })?.filter_map(|row| row.ok());

        let roms: HashSet<DataFile> = Vec::from_iter(roms_rows).drain(..).collect();
        Ok(Vec::from_iter(roms))
    }

    fn find_rom_usage<S>(&self, game_name: S, rom_name: S, rom_mode: RomsetMode) -> Result<RomSearch> where S: AsRef<str> + rusqlite::ToSql {
        let game_roms = self.get_romset_roms(game_name, rom_mode)?;
        
        let roms = game_roms.into_iter().filter(|rom| {
            rom.name.eq(rom_name.as_ref())
        }).collect();

        let rom_ids = DBReader::get_ids_from_files(self.conn, roms)?.found;

        self.find_sets_for_roms(rom_ids, rom_mode)
    }

    fn get_romset_shared_roms<S>(&self, game_name: S, rom_mode: RomsetMode) -> Result<RomSearch> where S: AsRef<str> + rusqlite::ToSql {
        let game_roms = self.get_romset_roms(game_name, rom_mode)?;

        let rom_ids = DBReader::get_ids_from_files(self.conn, game_roms)?.found;

        self.find_sets_for_roms(rom_ids, rom_mode)
    }

    fn get_romsets_from_roms(&self, roms: Vec<DataFile>, rom_mode: RomsetMode) -> Result<RomSearch> {
        let mut search_rom_ids_result = DBReader::get_ids_from_files(self.conn, roms)?;

        let mut rom_search = self.find_sets_for_roms(search_rom_ids_result.found, rom_mode)?;
        rom_search.unknowns.append(search_rom_ids_result.not_found.as_mut());
        Ok(rom_search)
    }

    fn get_devices_for_game<S>(&self, game_name: S) -> Result<Vec<String>> where S: AsRef<str> + rusqlite::ToSql {
        let mut search_stmt = self.conn.prepare("SELECT devices.device_ref FROM devices
            JOIN game_roms ON devices.device_ref = game_roms.game_name
            WHERE devices.game_name = ?1 GROUP BY devices.device_ref;")?;

        let result = search_stmt.query_map(params![game_name], |row| {
            Ok(row.get(0)?)
        })?.filter_map(|row| row.ok());

        Ok(result.collect())
    }

    fn get_file_checks(&self) -> Result<FileCheckSearch> {
        let mut stmt = self.conn.prepare("SELECT count(sha1), count(md5), count(crc) FROM roms;")?;
        let result = stmt.query_row(params![], |row| {
            Ok(FileCheckSearch {
                sha1: row.get(0)?,
                md5: row.get(1)?,
                crc: row.get(2)?,
            })
        })?;

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use std::{io::BufReader, fs::File, path::Path};
    use rusqlite::{Connection, OpenFlags};
    use crate::data::{importer::DatImporter, reader::sqlite::DBReader, models::file::FileType, writer::{sqlite::DBWriter}};
    use super::*;

    fn get_db_connection<'a, 'b>(dat_path: &'b impl AsRef<Path>) -> Result<Connection> {
        let mut conn = Connection::open_in_memory_with_flags(OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE)?;
        let writer = DBWriter::from_connection(&mut conn, 5);
        let mut importer = DatImporter::<BufReader<File>, DBWriter>::from_path(dat_path, writer)?;
        importer.load_dat()?;

        Ok(conn)
    }

    #[test]
    fn test_get_sets() -> Result<()> {
        let path = Path::new("testdata").join("test.dat");
        let conn = get_db_connection(&path)?;
        let data_reader = DBReader::from_connection(&conn);

        let data_files = data_reader.get_romset_roms(&"game1".to_string(), RomsetMode::Merged)?;
        assert_eq!(data_files.len(), 6);
        assert!(data_files.iter().find(|f| { f.name == "rom1.trom".to_string()} ).is_some());
        assert!(data_files.iter().find(|f| { f.name == "rom2.trom".to_string()} ).is_some());
        assert!(data_files.iter().find(|f| { f.name == "rom3.trom".to_string()} ).is_some());
        assert!(data_files.iter().find(|f| { f.name == "rom4.trom".to_string()} ).is_some());
        assert!(data_files.iter().find(|f| { f.name == "rom5.trom".to_string()} ).is_some());
        assert!(data_files.iter().find(|f| { f.name == "binfil1.bin".to_string()} ).is_some());

        let data_files = data_reader.get_romset_roms(&"game1".to_string(), RomsetMode::NonMerged)?;
        assert_eq!(data_files.len(), 4);
        assert!(data_files.iter().find(|f| { f.name == "rom1.trom".to_string()} ).is_some());
        assert!(data_files.iter().find(|f| { f.name == "rom2.trom".to_string()} ).is_some());
        assert!(data_files.iter().find(|f| { f.name == "rom3.trom".to_string()} ).is_some());
        assert!(data_files.iter().find(|f| { f.name == "binfil1.bin".to_string()} ).is_some());

        let data_files = data_reader.get_romset_roms(&"game1a".to_string(), RomsetMode::Split)?;
        assert_eq!(data_files.len(), 2);
        assert!(data_files.iter().find(|f| { f.name == "rom4.trom".to_string()} ).is_some());
        assert!(data_files.iter().find(|f| { f.name == "rom5.trom".to_string()} ).is_some());

        Ok(())
    }

    #[test]
    fn find_sets_for_roms() -> Result<()> {
        let path = Path::new("testdata").join("test.dat");
        let conn = get_db_connection(&path)?;
        let data_reader = DBReader::from_connection(&conn);

        let mut roms = vec![];
        let mut rom1 = DataFile::new("rom1", DataFileInfo::new(FileType::Rom));
        rom1.info.sha1 = Some("8bb3a81b9fa2de5163f0ffc634a998c455bcca25".to_string());
        roms.push(rom1);
        let mut rom2 = DataFile::new("rom2", DataFileInfo::new(FileType::Rom));
        rom2.info.sha1 = Some("802e076afc412be12db3cb8c79523f65d612a6cf".to_string());
        rom2.info.crc = Some("dc20b010".to_string());
        roms.push(rom2);

        let rom_sets = data_reader.get_romsets_from_roms(roms, RomsetMode::Merged)?;
        // TODO add validation
        println!("{:?}", rom_sets);

        let stats = data_reader.get_stats()?;
        // TODO add validation
        println!("{}", stats);

        Ok(())
    }

    #[test]
    fn find_rom_id_from_sha1() -> Result<()> {
        let path = Path::new("testdata").join("test.dat");
        let conn = get_db_connection(&path)?;

        let mut roms = vec![];
        let mut rom1 = DataFile::new("rom1", DataFileInfo::new(FileType::Rom));
        rom1.info.sha1 = Some("8bb3a81b9fa2de5163f0ffc634a998c455bcca25".to_string());
        roms.push(rom1);
        let result = DBReader::get_ids_from_files(&conn, roms)?;
        let rom_ids = result.found;
        let not_found = result.not_found;

        assert_eq!(rom_ids.len(), 1);
        assert_eq!(rom_ids[0].0, 2);
        assert_eq!(not_found.len(), 0);

        Ok(())
    }

    #[test]
    fn find_rom_id_from_sha1_and_crc() -> Result<()> {
        let path = Path::new("testdata").join("test.dat");
        let conn = get_db_connection(&path)?;

        let mut roms = vec![];
        let mut rom1 = DataFile::new("rom1", DataFileInfo::new(FileType::Rom));
        rom1.info.sha1 = Some("802e076afc412be12db3cb8c79523f65d612a6cf".to_string());
        rom1.info.crc = Some("dc20b010".to_string());
        roms.push(rom1);
        let result = DBReader::get_ids_from_files(&conn, roms)?;
        let rom_ids = result.found;
        let not_found = result.not_found;

        assert!(rom_ids.len() == 1);
        assert!(rom_ids[0].0 == 0);
        assert!(not_found.len() == 0);

        Ok(())
    }

    #[test]
    fn find_rom_id_from_sha1_and_crc_and_md5_but_no_md5_in_db() -> Result<()> {
        let path = Path::new("testdata").join("test.dat");
        let conn = get_db_connection(&path)?;

        let mut roms = vec![];
        let mut rom1 = DataFile::new("rom1", DataFileInfo::new(FileType::Rom));
        rom1.info.sha1 = Some("8273bfebe84dd41a5d237add8f9d03ac9bb0ef54".to_string());
        rom1.info.crc = Some("1b736d41".to_string());
        rom1.info.md5 = Some("0de4e413deb3ae71e9326d70df4d1a27".to_string());
        roms.push(rom1);
        let result = DBReader::get_ids_from_files(&conn, roms)?;
        let rom_ids = result.found;
        let not_found = result.not_found;

        assert_eq!(rom_ids.len(), 1);
        assert_eq!(rom_ids[0].0, 5);
        assert_eq!(not_found.len(), 0);

        Ok(())
    }

    #[test]
    fn dont_find_rom_id_from_sha1_and_crc_and_wrong_size() -> Result<()> {
        let path = Path::new("testdata").join("test.dat");
        let conn = get_db_connection(&path)?;

        let mut roms = vec![];
        let mut rom1 = DataFile::new("rom1", DataFileInfo::new(FileType::Rom));
        rom1.info.sha1 = Some("8273bfebe84dd41a5d237add8f9d03ac9bb0ef54".to_string());
        rom1.info.crc = Some("1b736d41".to_string());
        rom1.info.size = Some(1024);
        roms.push(rom1);
        let result = DBReader::get_ids_from_files(&conn, roms)?;
        let rom_ids = result.found;
        let not_found = result.not_found;

        assert!(rom_ids.len() == 0);
        assert!(not_found.len() == 1);

        Ok(())
    }

    #[test]
    fn get_devices_dependencies() -> Result<()> {
        let path = Path::new("testdata").join("test.dat");
        let conn = get_db_connection(&path)?;
        let data_reader = DBReader::from_connection(&conn);

        let devices = data_reader.get_devices_for_game(&"game1".to_string())?;
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0], "device1");

        Ok(())
    }
}