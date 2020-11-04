use anyhow::Result;
use log::{debug, error};
use rusqlite::{Connection, OpenFlags, params};

use crate::{data::models::{file::DataFile, game::Game}};

use super::DataWriter;

pub enum DBMode {
    Memory,
    File(String),
}

#[derive(Debug)]
pub struct DBWriter {
    conn: Connection,
}

impl DBWriter {
    pub fn new(mode: DBMode) -> Result<Self> {
        let connection = match mode {
            DBMode::Memory => Connection::open_in_memory()?,
            DBMode::File(p) => Connection::open_with_flags(p, OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE)?
        };

        Ok(Self { conn: connection })
    }

    fn conn(&self) -> &Connection {
        &self.conn
    }
    
    fn conn_mut(&mut self) -> &mut Connection {
        &mut self.conn
    }
    
    fn create_schema(&self) -> Result<()> {
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
        self.conn().execute( "CREATE INDEX checksums ON roms(sha1, md5, crc);", params![])?;

        // Machines/Games
        self.conn().execute(
            "CREATE TABLE games (
                id          INTEGER PRIMARY KEY,
                name        TEXT NOT NULL ON CONFLICT FAIL,
                clone_of    TEXT,
                rom_of      TEXT,
                source_file TEXT);",
            params![])?;
        // Indexes
        self.conn().execute( "CREATE INDEX name ON games(name);", params![])?;

        // Machine/Roms
        self.conn().execute(
            "CREATE TABLE game_roms (
                game_id     INTEGER,
                rom_id      INTEGER,
                name        TEXT,
                PRIMARY KEY (game_id, rom_id, name));",
            params![])?;
        // Indexes
        self.conn().execute( "CREATE INDEX game ON game_roms(game_id);", params![])?;
        self.conn().execute( "CREATE INDEX rom ON game_roms(rom_id);", params![])?;

        Ok(())
    }

    fn get_rom_ids(&self, roms: &[DataFile]) -> Result<Vec<(u32, String)>> {
        let mut rom_name_pair: Vec<(u32, String)> = vec![];

        let mut rom_stmt = self.conn().prepare("SELECT id FROM roms WHERE sha1 = ?1;")?;

        for rom in roms {
            match rom.name.to_owned() {
                None => { error!("Rom without a name, not adding it.") },
                Some(rom_name) => {
                    let rom_result: rusqlite::Result<u32> = rom_stmt.query_row(params![ rom.sha1 ], |row| {
                        Ok(row.get(0)?)
                    });

                    let r_id = match rom_result {
                        Ok(id) => { Ok(id) },
                        Err(rusqlite::Error::QueryReturnedNoRows) => {
                            // No Rom in the DB, we add it
                            self.conn().execute(
                                "INSERT INTO roms (sha1, md5, crc, size, status) VALUES (?1, ?2, ?3, ?4, ?5);",
                                params![ rom.sha1, rom.md5, rom.crc, rom.size, rom.status ])
                                .and_then(|_| {
                                    match self.conn().prepare("SELECT last_insert_rowid();") {
                                        Ok(mut rom_stmt) => {
                                            let rom_id = rom_stmt.query_row(params![], |row| {
                                                Ok(row.get(0)?)
                                            });
                                            rom_id
                                        }
                                        Err(e) => { Err(e) }
                                    }
                                })
                        },
                        Err(e) => Err(e)
                    };

                    match r_id {
                        Ok(id) => {
                            rom_name_pair.push((id, rom_name));
                        }
                        Err(e) => { error!("Error adding a rom: {}", e) }
                    }
                }
            }
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
            "INSERT INTO games (name, clone_of, rom_of, source_file) VALUES (?1, ?2, ?3, ?4);",
            params![game.name, game.clone_of, game.rom_of, game.source_file]);

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