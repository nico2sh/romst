use std::{iter::FromIterator, collections::HashSet, path::Path};

use anyhow::Result;
use rusqlite::{Connection, OpenFlags, params};

use crate::{RomsetMode, data::models::{game::Game, file::{DataFile, FileType::Rom}}};

use super::DataReader;

#[derive(Debug)]
pub struct DBReader {
    conn: Connection,
}

impl DBReader {
    pub fn new(db_file: &Path) -> Result<Self> {
        let conn = Connection::open_with_flags(db_file, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        Ok(Self { conn } )
    }

    pub fn from_connection(conn: Connection) -> Self {
        Self { conn }
    }

    fn get_clones_for(&self, game_name: &String) -> Result<Vec<DataFile>> {
        let mut roms_stmt = self.conn.prepare("SELECT game_roms.name, roms.sha1, roms.md5, roms.crc, roms.size, roms.status
            FROM games JOIN game_roms ON
            games.id = game_roms.game_id
            JOIN roms ON roms.id = game_roms.rom_id WHERE games.clone_of = ?1;")?;
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

impl DataReader for DBReader {
    fn get_game(&self, game_name: &String) -> Result<Game> {
        let mut game_stmt = self.conn.prepare("SELECT id, name, clone_of, rom_of, source_file, info_desc, info_year, info_manuf FROM games WHERE name = ?1;")?;
        let game: Game = game_stmt.query_row(params![ game_name ], |row| {
            Ok(
                Game {
                    name: row.get(1)?,
                    clone_of: row.get(2)?,
                    rom_of: row.get(3)?,
                    source_file: row.get(4)?,
                    info_description: row.get(5)?,
                    info_year: row.get(6)?,
                    info_manufacturer: row.get(7)?
                }
            )
        })?;

        Ok(game)
    }

    fn get_gameset_roms(&self, game_name: &String, rom_mode: &RomsetMode) -> Result<Vec<DataFile>> {
        let game = self.get_game(game_name)?;
        let mut roms_stmt = self.conn.prepare("SELECT game_roms.name, roms.sha1, roms.md5, roms.crc, roms.size, roms.status
            FROM games JOIN game_roms ON
            games.id = game_roms.game_id
            JOIN roms ON roms.id = game_roms.rom_id WHERE games.name = ?1;")?;

        let roms_rows = roms_stmt.query_map(params![ game.name ], |row| {
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

        let roms = Vec::from_iter(roms_rows);

        let clone_of = game.clone_of;

        match rom_mode {
            RomsetMode::Merged => {
                match clone_of {
                    Some(_) => {
                        // This is a clone, this rom should be empty as merged should be all in the parent
                        Ok(vec![])
                    },
                    None => {
                        let roms = self.get_clones_for(&game_name)?;
                        Ok(roms)
                    }
                }
            },
            RomsetMode::NonMerged => {
                Ok(roms)
            },
            RomsetMode::Split => {
                match clone_of {
                    Some(parent) => {
                        let parent_roms = self.get_gameset_roms(&parent, &RomsetMode::NonMerged)?;
                        let result = roms.into_iter().filter(|rom| {
                            !parent_roms.iter().position(|pr| {
                                let a = pr.eq(rom);
                                a
                            }).is_some()
                        });
                        Ok(Vec::from_iter(result))
                    },
                    None => Ok(roms)
                }
            },
        }
    }
}
