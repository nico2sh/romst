use std::path::{Path, PathBuf};

use anyhow::Result;
use log::{debug, error, warn};
use rusqlite::{Connection, OpenFlags, ToSql, params};

use crate::{data::models::{file::DataFile, game::Game}};

use super::DataWriter;

pub enum DBMode {
    Memory,
    File(PathBuf),
}

#[derive(Debug)]
pub struct DBWriter {
    conn: Connection,
    rom_row_id: u32,
}

impl DBWriter {
    pub fn new(mode: DBMode) -> Result<Self> {
        let connection = match mode {
            DBMode::Memory => Connection::open_in_memory()?,
            DBMode::File(p) => Connection::open_with_flags(p, OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE)?
        };

        Ok(Self { conn: connection, rom_row_id: 0 })
    }

    fn conn(&self) -> &Connection {
        &self.conn
    }
    
    fn conn_mut(&mut self) -> &mut Connection {
        &mut self.conn
    }
    
    fn create_schema(&self) -> Result<()> {
        self.conn().execute(
            "CREATE TABLE info (
                name        TEXT,
                description TEXT,
                version     TEXT);", 
            params![])?;

        debug!("Creating ROMS table");
        // Rom
        self.conn().execute(
            "CREATE TABLE roms (
                id      INTEGER PRIMARY KEY,
                sha1    TEXT,
                md5     TEXT,
                crc     TEXT,
                size    INT,
                status  TEXT);", 
            params![])?;
        debug!("Creating ROMS indexes");
        // Indexes
        self.conn().execute( "CREATE INDEX sha1 ON roms(sha1);", params![])?;
        self.conn().execute( "CREATE INDEX md5 ON roms(md5);", params![])?;
        self.conn().execute( "CREATE INDEX crc ON roms(crc);", params![])?;
        self.conn().execute( "CREATE INDEX checks ON roms(sha1, md5, crc);", params![])?;

        debug!("Creating Games table");
        // Machines/Games
        self.conn().execute(
            "CREATE TABLE games (
                id          INTEGER PRIMARY KEY,
                name        TEXT NOT NULL ON CONFLICT FAIL,
                clone_of    TEXT,
                rom_of      TEXT,
                source_file TEXT,
                info_desc   TEXT,
                info_year   TEXT,
                info_manuf  TEXT);",
            params![])?;
        debug!("Creating Games indexes");
        // Indexes
        self.conn().execute( "CREATE INDEX name ON games(name);", params![])?;

        debug!("Creating Games/ROMs table");
        // Machine/Roms
        self.conn().execute(
            "CREATE TABLE game_roms (
                game_id     INTEGER,
                rom_id      INTEGER,
                name        TEXT,
                PRIMARY KEY (game_id, rom_id, name));",
            params![])?;
        debug!("Creating Games/ROMs indexes");
        // Indexes
        self.conn().execute( "CREATE INDEX game ON game_roms(game_id);", params![])?;
        self.conn().execute( "CREATE INDEX rom ON game_roms(rom_id);", params![])?;

        Ok(())
    }

    fn get_rom_ids(&mut self, roms: &[DataFile]) -> Result<Vec<(u32, String)>> {
        let mut rom_name_pair: Vec<(u32, String)> = vec![];
        let mut roms_to_insert = vec![];

        for rom in roms {
            match rom.name.to_owned() {
                None => { error!("Rom without a name, not adding it.") },
                Some(rom_name) => {
                    let mut params: Vec<&dyn ToSql> = vec![];
                    let mut statement_where = vec![];
                    let mut param_num = 1;
                    if rom.sha1.is_some() {
                        params.push(rom.sha1.as_ref().unwrap());
                        statement_where.push(format!("sha1 = ?{}", param_num));
                        param_num = param_num + 1;
                    }
                    
                    if rom.md5.is_some() {
                        params.push(rom.md5.as_ref().unwrap());
                        statement_where.push(format!("md5 = ?{}", param_num));
                        param_num = param_num + 1;
                    }

                    if rom.crc.is_some() {
                        params.push(rom.crc.as_ref().unwrap());
                        statement_where.push(format!("crc = ?{}", param_num));
                        param_num = param_num + 1;
                    }

                    if rom.size.is_some() {
                        params.push(rom.size.as_ref().unwrap());
                        statement_where.push(format!("size = ?{}", param_num));
                    }

                    let statement = "SELECT id FROM roms WHERE ".to_string() + &statement_where.join(" AND ") + ";";

                    let mut rom_stmt = self.conn().prepare_cached(&statement)?;
                    let rom_result: rusqlite::Result<u32> = rom_stmt.query_row(params, |row| {
                        Ok(row.get(0)?)
                    });

                    let r_id = match rom_result {
                        Err(rusqlite::Error::QueryReturnedNoRows) => {
                            roms_to_insert.push(rom);
                            Ok(None)
                        },
                        Ok(id) => Ok(Some(id)),
                        Err(e) => Err(e),
                    };

                    match r_id {
                        Ok(o_id) => {
                            match o_id {
                                Some(id) => { rom_name_pair.push((id, rom_name)); }
                                None => {}
                            }
                        }
                        Err(e) => { error!("Error adding a rom: {}", e) }
                    }
                }
            }
        }

        let mut rom_row_id = self.rom_row_id;
        let tx = self.conn_mut().transaction()?;

        for rom in roms_to_insert {
            tx.execute(
                "INSERT INTO roms (id, sha1, md5, crc, size, status) VALUES (?1, ?2, ?3, ?4, ?5, ?6);",
                params![ rom_row_id, rom.sha1, rom.md5, rom.crc, rom.size, rom.status ])
                .and_then(|i| {
                    rom_name_pair.push((rom_row_id, rom.name.to_owned().unwrap()));
                    rom_row_id = rom_row_id + i as u32;
                    Ok(())
                })?;
        }
        match tx.commit() {
            Ok(_) => { self.rom_row_id = rom_row_id }
            Err(e) => { error!("Error inserting roms: {}", e) }
        }

        Ok(rom_name_pair)
    }
}

impl DataWriter for DBWriter {
    fn init(&self) -> Result<()> {
        self.create_schema()
    }
    
    fn on_new_game(&self, game: &Game) -> Result<()> {
        let result = self.conn().execute(
            "INSERT INTO games (name, clone_of, rom_of, source_file, info_desc, info_year, info_manuf) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7);",
            params![game.name, game.clone_of, game.rom_of, game.source_file, game.info_description, game.info_year, game.info_manufacturer]);

        match result {
            Ok(n) => { debug!("Inserted {} rows, game {}", n, game) }
            Err(e) => { error!("Error inserting row: {} for game {}", e, game) }
        }

        Ok(())
    } 

    fn on_new_roms(&mut self, game: &Game, roms: &[DataFile]) -> Result<()> {
        let game_id: u32 = {
            let mut game_stmt = self.conn().prepare("SELECT id, name FROM games WHERE name = ?1;")?;

            game_stmt.query_row(params![ game.name ], |row| {
                Ok(row.get(0)?)
            })?
        };

        let rom_list = self.get_rom_ids(roms);
        match rom_list {
            Ok(rom_id_names) => {
                let tx = self.conn_mut().transaction()?;
                for rom_id_name in rom_id_names {
                    let result = tx.execute(
                        "INSERT INTO game_roms (game_id, rom_id, name) VALUES (?1, ?2, ?3);",
                        params![ game_id, rom_id_name.0, rom_id_name.1 ] );
                    match result {
                        Ok(_n) => { debug!("Inserted a rom to a game") }
                        Err(e) => { error!("Error adding rom `{}` to the game {}: {}", rom_id_name.1, "", e) }
                    }
                }
                tx.commit()?;
            }
            Err(e) => { error!("Error retrieving and inserting roms: {}", e) }
        }

        Ok(())
    }

}