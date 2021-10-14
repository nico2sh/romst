pub mod sqlite;

use anyhow::Result;

use super::models::{dat_info::DatInfo, disk::GameDisk, file::*, game::Game};

pub trait DataWriter {
    fn init(&self) -> Result<()>;
    fn on_new_entry(&mut self, game: Game, roms: Vec<DataFile>, disks: Vec<GameDisk>, samples: Vec<String>, device_refs: Vec<String>) -> Result<()>;
    fn on_dat_info(&mut self, dat_info: DatInfo) -> Result<()>;
    fn finish(&mut self) -> Result<()>;
}