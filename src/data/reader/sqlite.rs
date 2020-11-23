use std::{collections::HashSet, iter::FromIterator, collections::HashMap, path::Path};

use anyhow::Result;
use rusqlite::{Connection, OpenFlags, params};

use crate::{RomsetMode, data::models::{file::{DataFile, FileType::Rom}, game::Game, set::GameSet, report::Report}};

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
}

impl <'d> DataReader for DBReader<'d> {
    fn get_game(&self, game_name: &String) -> Result<Game> {
        let mut game_stmt = self.conn.prepare("SELECT name, clone_of, rom_of, source_file, info_desc, info_year, info_manuf
            FROM games WHERE name = ?1;")?;
        let game: Game = game_stmt.query_row(params![ game_name ], |row| {
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
        })?;

        Ok(game)
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

    fn find_rom_usage(&self, game_name: &String, rom_name: &String) -> Result<Vec<Report>> {
        let mut roms_stmt = self.conn.prepare("SELECT DISTINCT game_roms.game_name, game_roms.name as romname
            FROM game_roms 
                WHERE game_roms.rom_id = (SELECT game_roms.rom_id FROM game_roms 
                WHERE game_roms.game_name = ?1 AND game_roms.name = ?2) ORDER BY game_roms.game_name;
        ")?;
        let roms_rows = roms_stmt.query_map(params![ game_name, rom_name ], |row| {
            Ok((row.get(0)?,
                row.get(1)?))
        })?.filter_map(|row| row.ok());

        let mut rows = roms_rows.peekable();
        let mut result: Vec<Report> = vec![];
        if let Some(row) = rows.peek() {
            let mut report = Report::new(String::from(&row.0), vec![], vec![]);
            for item in rows {
                let game_name = item.0;
                let rom_name = item.1;

                if report.name != game_name {
                    result.push(report);
                    report = Report::new(game_name, vec![ rom_name ], vec![]);
                } else {
                    report.add_having(rom_name);
                }
            }
        }

        Ok(result)
    }

    fn get_romset_shared_roms(&self, game_name: &String) -> Result<Vec<Report>> {
        let mut roms_stmt = self.conn.prepare("SELECT DISTINCT game_roms.game_name, game_roms.name as romname
            FROM game_roms WHERE game_roms.rom_id IN (SELECT game_roms.rom_id FROM game_roms 
                WHERE game_roms.game_name = ?1) ORDER BY game_roms.game_name;
        ")?;
        let roms_rows = roms_stmt.query_map(params![ game_name ], |row| {
            Ok((row.get(0)?,
                row.get(1)?))
        })?.filter_map(|row| row.ok());

        let mut rows = roms_rows.peekable();
        let mut result: Vec<Report> = vec![];
        if let Some(row) = rows.peek() {
            let mut report = Report::new(String::from(&row.0), vec![], vec![]);
            for item in rows {
                let game_name = item.0;
                let rom_name = item.1;

                if report.name != game_name {
                    result.push(report);
                    report = Report::new(game_name, vec![ rom_name ], vec![]);
                } else {
                    report.add_having(rom_name);
                }
            }
        }

        Ok(result)
    }
}
