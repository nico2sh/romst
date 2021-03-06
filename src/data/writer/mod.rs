pub mod sqlite;

use anyhow::Result;

use super::models::{disk::GameDisk, file::*, game::Game};

pub trait DataWriter {
    fn init(&self) -> Result<()>;
    fn on_new_entry(&mut self, game: Game, roms: Vec<DataFile>, disks: Vec<GameDisk>, samples: Vec<String>, device_refs: Vec<String>) -> Result<()>;
    fn finish(&mut self) -> Result<()>;
}