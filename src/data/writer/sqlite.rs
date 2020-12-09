use std::{iter::FromIterator, collections::HashMap, path::Path};

use anyhow::Result;
use log::{debug, error};
use rusqlite::{Connection, OpenFlags, ToSql, params};

use crate::{data::models::{file::DataFile, game::Game}};
use super::DataWriter;

#[derive(Debug)]
pub struct IdsCounter {
    pub rom: u32,
}

impl IdsCounter {
    pub fn new() -> Self { Self { rom: 0 } }
}


#[derive(Debug)]
pub struct DBWriter<'d> {
    conn: &'d mut Connection,
    ids: IdsCounter,
    game_buffer: HashMap<String, Game>,
    buffer_size: u16,
}

impl <'d> DBWriter<'d> {
    pub fn from_connection(conn: &'d mut Connection, buffer_size: u16) -> Self {
        Self { conn, ids: IdsCounter::new(), game_buffer: HashMap::new(), buffer_size }
    }

    fn create_schema(&self) -> Result<()> {
        self.conn.execute(
            "CREATE TABLE info (
                name        TEXT,
                description TEXT,
                version     TEXT);", 
            params![])?;

        debug!("Creating ROMS table");
        // Rom
        self.conn.execute(
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
        self.conn.execute( "CREATE INDEX sha1 ON roms(sha1);", params![])?;
        self.conn.execute( "CREATE INDEX md5 ON roms(md5);", params![])?;
        self.conn.execute( "CREATE INDEX crc ON roms(crc);", params![])?;
        self.conn.execute( "CREATE INDEX checks ON roms(sha1, md5, crc);", params![])?;

        debug!("Creating Games table");
        // Machines/Games
        self.conn.execute(
            "CREATE TABLE games (
                name        TEXT PRIMARY KEY,
                clone_of    TEXT,
                rom_of      TEXT,
                source_file TEXT,
                info_desc   TEXT,
                info_year   TEXT,
                info_manuf  TEXT);",
            params![])?;
        debug!("Creating Games indexes");
        // Indexes
        self.conn.execute( "CREATE INDEX parents ON games(clone_of);", params![])?;

        debug!("Creating Games/ROMs table");
        // Machine/Roms
        self.conn.execute(
            "CREATE TABLE game_roms (
                game_name   TEXT,
                rom_id      INTEGER,
                name        TEXT,
                parent      TEXT,
                PRIMARY KEY (game_name, rom_id, name));",
            params![])?;
        debug!("Creating Games/ROMs indexes");
        // Indexes
        self.conn.execute( "CREATE INDEX game ON game_roms(game_name);", params![])?;
        self.conn.execute( "CREATE INDEX rom ON game_roms(rom_id);", params![])?;
        self.conn.execute( "CREATE INDEX rom_parents ON game_roms(parent);", params![])?;

        Ok(())
    }

    fn get_rom_ids(&mut self, roms: Vec<DataFile>) -> Result<Vec<(u32, String)>> {
        let mut rom_name_pair: Vec<(u32, String)> = vec![];
        let mut roms_to_insert = vec![];

        for rom in roms {
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

            let statement = "SELECT id FROM roms WHERE ".to_string() +
                &statement_where.join(" AND ") + ";";

            let mut rom_stmt = self.conn.prepare_cached(&statement)?;
            let rom_result: rusqlite::Result<u32> = rom_stmt.query_row(params, |row| {
                Ok(row.get(0)?)
            });

            match rom_result {
                Err(rusqlite::Error::QueryReturnedNoRows) => {
                    roms_to_insert.push(rom);
                },
                Ok(id) => {
                    rom_name_pair.push((id, rom.name));
                },
                Err(e) => error!("Error adding a rom: {}", e),
            };
        }

        let mut rom_row_id = self.ids.rom;
        let tx = self.conn.transaction()?;

        for rom in roms_to_insert {
            tx.execute(
                "INSERT INTO roms (id, sha1, md5, crc, size, status) VALUES (?1, ?2, ?3, ?4, ?5, ?6);",
                params![ rom_row_id, rom.sha1, rom.md5, rom.crc, rom.size, rom.status ])
                .and_then(|i| {
                    rom_name_pair.push((rom_row_id, rom.name));
                    rom_row_id = rom_row_id + i as u32;
                    Ok(())
                })?;
        }
        match tx.commit() {
            Ok(_) => { self.ids.rom = rom_row_id }
            Err(e) => { error!("Error inserting roms: {}", e) }
        }

        // We remove the duplicates
        rom_name_pair.sort();
        rom_name_pair.dedup();
        Ok(rom_name_pair)
    }

    fn add_game_to_buffer(&mut self, game: Game) {
        let game_name = game.name.to_owned();
        self.game_buffer.insert(game_name, game); 
    }

    fn write_game_buffer(&mut self) -> Result<()> {
        let tx = self.conn.transaction()?;
        let buffer = &self.game_buffer;
        let values = buffer.values();
        for value in values {
            let game = value;
            let p = params![game.name,
                game.clone_of,
                game.rom_of,
                game.source_file,
                game.info_description,
                game.info_year,
                game.info_manufacturer];
            match tx.execute("INSERT INTO games (name, clone_of, rom_of, source_file, info_desc, info_year, info_manuf)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7);",
                p) {
                    Ok(_) => {}
                    Err(e) => { error!("Error inserting row in the games db: {}", e) }
                }
        }

        tx.commit()?;
        self.game_buffer.clear();

        Ok(())
    }

    fn get_roms_from_parents(&mut self) -> Result<Vec<(String, u32, String)>>{
        let mut stmt = self.conn.prepare("SELECT games.name AS game_name, game_roms.rom_id, game_roms.game_name as parent, game_roms.name FROM game_roms
            JOIN games ON games.clone_of = game_roms.game_name
            WHERE games.clone_of IS NOT NULL;")?;
        let rows = stmt.query_map(params![], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })?.filter_map(|row| row.ok());

        Ok(Vec::from_iter(rows))
    }
}

impl <'d> DataWriter for DBWriter<'d> {
    fn init(&self) -> Result<()> {
        self.create_schema()
    }
    
    fn on_new_game(&mut self, game: Game) -> Result<()> {
        self.add_game_to_buffer(game);

        if self.game_buffer.len() as u16 >= self.buffer_size {
            self.write_game_buffer()?;
        }

        Ok(())
    } 

    fn on_new_roms(&mut self, game: Game, roms: Vec<DataFile>) -> Result<()> {
        let rom_list = self.get_rom_ids(roms);
        match rom_list {
            Ok(rom_id_names) => {
                let tx = self.conn.transaction()?;
                for rom_id_name in rom_id_names {
                    let result = tx.execute(
                        "INSERT INTO game_roms (game_name, rom_id, name) VALUES (?1, ?2, ?3);",
                        params![ game.name, rom_id_name.0, rom_id_name.1 ] );
                    match result {
                        Ok(_n) => { debug!("Inserted rom {} with id {} to the game {}", rom_id_name.1, rom_id_name.0, game.name) }
                        Err(e) => { error!("Error adding rom `{}` to the game {}: {}", rom_id_name.1, "", e) }
                    }
                }
                tx.commit()?;
            }
            Err(e) => { error!("Error retrieving and inserting roms: {}", e) }
        }

        Ok(())
    }

    fn finish(&mut self) -> Result<()> {
        self.write_game_buffer()?;
        let roms_from_parents = self.get_roms_from_parents()?;

        let tx = self.conn.transaction()?;
        for item in roms_from_parents {
            let game_name = item.0;
            let rom_id = item.1;
            let parent = item.2;

            let result = tx.execute("UPDATE game_roms SET parent = ?1
                WHERE game_roms.game_name = ?2 AND game_roms.rom_id = ?3;", params![parent, game_name, rom_id])?;
            if result > 1 {
                debug!("Updated {} rows, should be only 1 for game {}, rom_id {}, with parent {}, unless is a 'nodump'", result, game_name, rom_id, parent);
            }
        }
        tx.commit()?;

        Ok(())
    }
}