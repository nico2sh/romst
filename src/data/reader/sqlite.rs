use std::{collections::HashSet, fmt::Display, iter::FromIterator};

use anyhow::Result;
use console::Style;
use log::{debug, error, warn};
use rusqlite::{Connection, Row, ToSql, params};
use serde::{Deserialize, Serialize};

use crate::{RomsetMode, data::models::{disk::GameDisk, file::{DataFile, DataFileInfo, FileType}, game::Game}};

use super::{DataReader, DbDataEntry, FileCheckSearch, RomSearch, SetDependencies};

#[derive(Debug)]
pub struct SearchEntryIds<T> {
    pub found: Vec<DbDataEntry<T>>,
    pub not_found: Vec<T>,
    pub ignored: Vec<T>
}

impl <T> SearchEntryIds<T> {
    fn new() -> Self { Self { found: vec![], not_found: vec![], ignored: vec![] } }

    fn add_found(&mut self, id: u32, file: T) {
        self.found.push(DbDataEntry::new(id, file));
    }

    fn add_not_found(&mut self, entry: T) {
        self.not_found.push(entry);
    }
}

#[derive(Serialize, Deserialize)]
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

impl Default for DBReport {
    fn default() -> Self {
        DBReport::new()
    }
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
Game rom_of = row.get(9)?;
Game source_file = row.get(10)?;
Game sample_of = row.get(11)?;
Game info_desc = row.get(12)?;
Game info_year = row.get(13)?;
Game info_manuf = row.get(14)?;
Rom id = row.get(15)?;
*/
const ROMS_QUERY: &str = "SELECT DISTINCT game_roms.game_name, game_roms.name as rom_name, roms.sha1, roms.md5, roms.crc, roms.size, game_roms.status, game_roms.parent, games.clone_of, games.rom_of, games.source_file, games.sample_of, games.info_desc, games.info_year, games.info_manuf, roms.id
                FROM game_roms JOIN roms ON game_roms.rom_id = roms.id JOIN games ON game_roms.game_name = games.name";

fn process_row(row: &Row) -> Result<(Game, DbDataEntry<DataFile>, Option<String>), rusqlite::Error> {
    let mut game = Game::new(row.get(0)?);
    game.clone_of = row.get(8)?;
    game.rom_of = row.get(9)?;
    game.source_file = row.get(10)?;
    game.sample_of = row.get(11)?;
    game.info_description = row.get(12)?;
    game.info_year = row.get(13)?;
    game.info_manufacturer = row.get(14)?;

    let mut data_file_info = DataFileInfo::new(FileType::Rom);
    data_file_info.sha1 = row.get(2)?;
    data_file_info.md5 = row.get(3)?;
    data_file_info.crc = row.get(4)?;
    data_file_info.size = row.get(5)?;

    let rom_name: String = row.get(1)?;
    let mut data_file = DataFile::new(rom_name, data_file_info);
    data_file.status = row.get(6)?;

    let rom_id = row.get(15)?;
    let db_entry = DbDataEntry::new(rom_id, data_file);

    let rom_parent: Option<String> = row.get(7)?;

    Ok((game, db_entry, rom_parent))
}

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

        Ok(db_report)
    }

    fn find_sets_for_roms(&self, db_roms: Vec<DbDataEntry<DataFile>>, rom_mode: RomsetMode) -> Result<RomSearch> {
        let mut params: Vec<&dyn ToSql> = vec![];
        let mut ids_cond = String::new();
        let mut i = 0;
        db_roms.iter().for_each(|db_rom| {
            if i != 0 {
                ids_cond.push_str(", ");
            }
            i += 1;                     
            ids_cond.push_str("?");
            ids_cond.push_str(&i.to_string());
            params.push(&db_rom.id);
        });

        // We do a query with all the roms we received, the result will return all sets associated with these roms
        let query = ROMS_QUERY.to_string() + " WHERE game_roms.rom_id IN (" + &ids_cond + ") ORDER BY game_roms.game_name;";

        type QueryResult = (Game, DbDataEntry<DataFile>, Option<String>);
        let mut roms_stmt = self.conn.prepare(&query)?;
        let roms_rows = roms_stmt.query_map::<QueryResult, _, _>(params, |row| {
            process_row(row)
        })?.filter_map(|result| {
            // We filter the erros
            result.ok()
        }).flat_map(|results| {
            // Since we can have more than one rom id with different name, we create a vec with each name
            // Most of the times it will be only one
            let rom_id: u32 = results.1.id;
            db_roms.iter().filter_map(|db_rom| {
                if rom_id == db_rom.id {
                    Some(db_rom.file.name.clone())
                } else {
                    None
                }
            }).map(|file_name| {
                let game = results.0.clone();
                let mut data_file = results.1.file.clone();
                data_file.name = file_name;

                (game,
                DbDataEntry::new(rom_id, data_file),
                results.2.clone())
            }).collect::<Vec<_>>()
        }).collect::<Vec<_>>();

        let mut result = RomSearch::new();
        for item in roms_rows {
            let game = item.0;
            let rom = item.1;
            let game_parent = item.2;

            match rom_mode {
                RomsetMode::Merged => {
                    if let Some(game_parent_name) = game.rom_of {
                        result.add_file_for_set(game_parent_name, rom);
                    } else {
                        result.add_file_for_set(game.name, rom);
                    }
                }
                RomsetMode::NonMerged => {
                    result.add_file_for_set(game.name, rom);
                }
                RomsetMode::Split => {
                    if game_parent == None {
                        result.add_file_for_set(game.name, rom);
                    }
                }
            }
        }

        Ok(result)
    }

    pub fn get_ids_from_files(conn: &Connection, files: Vec<DataFile>) -> Result<SearchEntryIds<DataFile>> {
        let mut result = SearchEntryIds::new();
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

    pub fn get_ids_from_disks(conn: &Connection, files: Vec<GameDisk>) -> Result<SearchEntryIds<GameDisk>> {
        let mut result = SearchEntryIds::new();
        for file in files {
            match &file.info.status {
                Some(status) if status.to_lowercase() == "nodump" => {
                    // We ignore the ones without dump
                    result.ignored.push(file);
                },
                _ => {
                    let mut params: Vec<(&str, &dyn ToSql)> = vec![];
                    let mut statement_where = vec![];
                    let mut has_hash = false;

                    if let Some(ref sha1) = file.info.sha1 {
                        has_hash = true;
                        params.push((":sha1", sha1));
                        statement_where.push("(sha1 = :sha1 OR sha1 IS NULL)");
                    }

                    if !has_hash {
                        warn!("File `{}` has no hash value, it could match any other rom, should be ignored", file);
                        result.not_found.push(file);
                    } else {
                        // Minimum fields to find, has to have at least md5 or sha1
                        statement_where.push("(sha1 IS NOT NULL)");

                        let statement = "SELECT id FROM disks WHERE ".to_string() +
                            &statement_where.join(" AND ") + ";";
                        
                        let mut rom_stmt = conn.prepare_cached(&statement)?;
                        let query_rom_result: Vec<u32> = rom_stmt.query_map_named(params.as_slice(), |row| {
                            Ok(row.get(0)?)
                        })?.filter_map(|row| row.ok() ).collect();

                        match query_rom_result.len() {
                            0 => {
                                debug!("No disk found in DB: {}", file);
                                result.add_not_found(file);
                            },
                            1 => {
                                debug!("Found disk in DB: {}", file);
                                result.add_found(query_rom_result[0], file);
                            },
                            n => {
                                // TODO: There is a corner case which is, if the search has a sha1, and the DB has a md5 it may match as both with match against the null value
                                warn!("Found more than one rom ({}) on the query, ROM: {}", n, file);
                                result.ignored.push(file);
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

    fn get_romset_roms<S>(&self, game_name: S, rom_mode: RomsetMode) -> Result<Vec<DbDataEntry<DataFile>>> where S: AsRef<str> + rusqlite::ToSql {
        let mut query = ROMS_QUERY.to_string();
        match rom_mode {
            RomsetMode::Merged => {
                query.push_str(" WHERE (game_roms.game_name = ?1 OR games.rom_of = ?1);");
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
            let r = process_row(row)?;
            Ok(r.1)
        })?.filter_map(|row| row.ok());

        let roms: HashSet<DbDataEntry<DataFile>> = Vec::from_iter(roms_rows).drain(..).collect();
        Ok(Vec::from_iter(roms))
    }

    fn find_rom_usage<S>(&self, game_name: S, rom_name: S, rom_mode: RomsetMode) -> Result<RomSearch> where S: AsRef<str> + rusqlite::ToSql {
        let game_roms = self.get_romset_roms(game_name, rom_mode)?;
        
        let roms = game_roms.into_iter().filter_map(|rom| {
            if rom.file.name.eq(rom_name.as_ref()) {
                Some(rom.file)
            } else {
                None
            }
        }).collect();

        let rom_ids = DBReader::get_ids_from_files(self.conn, roms)?.found;

        self.find_sets_for_roms(rom_ids, rom_mode)
    }

    fn get_romset_shared_roms<S>(&self, game_name: S, rom_mode: RomsetMode) -> Result<RomSearch> where S: AsRef<str> + rusqlite::ToSql {
        let game_roms = self.get_romset_roms(game_name, rom_mode)?;

        //let rom_ids = DBReader::get_ids_from_files(self.conn, game_roms)?.found;

        self.find_sets_for_roms(game_roms, rom_mode)
    }

    fn get_romsets_from_roms(&self, roms: Vec<DataFile>, rom_mode: RomsetMode) -> Result<RomSearch> {
        let mut search_rom_ids_result = DBReader::get_ids_from_files(self.conn, roms)?;

        let mut rom_search = self.find_sets_for_roms(search_rom_ids_result.found, rom_mode)?;
        rom_search.unknowns.append(search_rom_ids_result.not_found.as_mut());
        Ok(rom_search)
    }

    fn get_devices_for_game<S>(&self, game_name: S) -> Result<SetDependencies> where S: AsRef<str> + rusqlite::ToSql {
        let mut search_stmt = self.conn.prepare("SELECT devices.device_ref FROM devices
            JOIN game_roms ON devices.device_ref = game_roms.game_name
            WHERE devices.game_name = ?1 GROUP BY devices.device_ref;")?;

        let result = search_stmt.query_map(params![game_name], |row| {
            Ok(row.get(0)?)
        })?.filter_map(|row| row.ok());

        let mut set_dependencies = SetDependencies::new(game_name.as_ref());
        set_dependencies.dependencies = result.collect();

        Ok(set_dependencies)
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
    use crate::data::{importer::DatImporter, models::{disk::GameDiskInfo, file::FileType}, reader::sqlite::DBReader, writer::{sqlite::DBWriter}};
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
        assert!(data_files.iter().find(|f| { f.file.name == "rom1.trom".to_string()} ).is_some());
        assert!(data_files.iter().find(|f| { f.file.name == "rom2.trom".to_string()} ).is_some());
        assert!(data_files.iter().find(|f| { f.file.name == "rom3.trom".to_string()} ).is_some());
        assert!(data_files.iter().find(|f| { f.file.name == "rom4.trom".to_string()} ).is_some());
        assert!(data_files.iter().find(|f| { f.file.name == "rom5.trom".to_string()} ).is_some());
        assert!(data_files.iter().find(|f| { f.file.name == "binfil1.bin".to_string()} ).is_some());

        let data_files = data_reader.get_romset_roms(&"game1".to_string(), RomsetMode::NonMerged)?;
        assert_eq!(data_files.len(), 4);
        assert!(data_files.iter().find(|f| { f.file.name == "rom1.trom".to_string()} ).is_some());
        assert!(data_files.iter().find(|f| { f.file.name == "rom2.trom".to_string()} ).is_some());
        assert!(data_files.iter().find(|f| { f.file.name == "rom3.trom".to_string()} ).is_some());
        assert!(data_files.iter().find(|f| { f.file.name == "binfil1.bin".to_string()} ).is_some());

        let data_files = data_reader.get_romset_roms(&"game1a".to_string(), RomsetMode::Split)?;
        assert_eq!(data_files.len(), 2);
        assert!(data_files.iter().find(|f| { f.file.name == "rom4.trom".to_string()} ).is_some());
        assert!(data_files.iter().find(|f| { f.file.name == "rom5.trom".to_string()} ).is_some());

        let data_files = data_reader.get_romset_roms(&"game4".to_string(), RomsetMode::Split)?;
        assert_eq!(data_files.len(), 4);
        assert!(data_files.iter().find(|f| { f.file.name == "rrham.rom".to_string()} ).is_some());
        assert!(data_files.iter().find(|f| { f.file.name == "rhum1.rom".to_string()} ).is_some());
        assert!(data_files.iter().find(|f| { f.file.name == "rhum2.rom".to_string()} ).is_some());
        assert!(data_files.iter().find(|f| { f.file.name == "rhin1.rom".to_string()} ).is_some());

        Ok(())
    }

    #[test]
    fn test_rom_ids_retrieval_with_repeated_roms() -> Result<()> {
        let path = Path::new("testdata").join("test.dat");
        let conn = get_db_connection(&path)?;
        let mut roms = vec![];
        let mut rom1 = DataFile::new("rrham.rom", DataFileInfo::new(FileType::Rom));
        rom1.info.sha1 = Some("b47a81a4bce8d7abd8f940b0acb5674776b4ae03".to_string());
        rom1.info.crc = Some("7182d83b".to_string());
        roms.push(rom1);
        let mut rom2 = DataFile::new("rhum1.rom", DataFileInfo::new(FileType::Rom));
        rom2.info.sha1 = Some("0d0410009c5bd3802b0021c8f29edc997a83c88c".to_string());
        rom2.info.crc = Some("4bec6e65".to_string());
        roms.push(rom2);
        let mut rom3 = DataFile::new("rhum2.rom", DataFileInfo::new(FileType::Rom));
        rom3.info.sha1 = Some("0d0410009c5bd3802b0021c8f29edc997a83c88c".to_string());
        rom3.info.crc = Some("4bec6e65".to_string());
        roms.push(rom3);
        let mut rom4 = DataFile::new("rhin1.rom", DataFileInfo::new(FileType::Rom));
        rom4.info.sha1 = Some("5bef439d1d775e0ff3a17189478e87b3dd5d0e49".to_string());
        rom4.info.crc = Some("0d46fa2d".to_string());
        roms.push(rom4);

        let rom_ids = DBReader::get_ids_from_files(&conn, roms)?.found;

        let roms = rom_ids.len();
        assert_eq!(4, roms);

        let data_reader = DBReader::from_connection(&conn);
        let rom_search = data_reader.find_sets_for_roms(rom_ids, RomsetMode::Split)?;

        assert_eq!(1, rom_search.set_results.len());
        assert!(rom_search.set_results.get("game4").is_some());
        let game4 = rom_search.set_results.get("game4").unwrap();
        assert_eq!(4, game4.roms_included.len());

        Ok(())
    }

    #[test]
    fn get_disk_id_retrieval() -> Result<()> {
        let path = Path::new("testdata").join("test.dat");
        let conn = get_db_connection(&path)?;

        let mut files = vec![];
        let mut disk1 = GameDisk::new("gm5-001.chd");
        let mut disk1_info = GameDiskInfo::new();
        disk1_info.sha1 = Some("0f8eb9bb79efdc84dfdb46e2a1c123dd5a7dd221".to_string());
        disk1_info.region = Some("cdrom".to_string());
        disk1.info = disk1_info;
        files.push(disk1);

        let disks_ids = DBReader::get_ids_from_disks(&conn, files)?;

        assert_eq!(1, disks_ids.found.len());
        assert_eq!(0, disks_ids.not_found.len());
        assert_eq!(1, disks_ids.found[0].id);
        assert_eq!("gm5-001.chd", disks_ids.found[0].file.name);

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
        assert!(rom_ids[0].id == 2);
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
        assert!(rom_ids[0].id == 0);
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
        assert!(rom_ids[0].id == 5);
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
        assert_eq!(devices.dependencies.len(), 1);
        assert_eq!(devices.dependencies[0], "device1");

        Ok(())
    }
}