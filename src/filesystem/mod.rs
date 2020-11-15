mod sha1;
mod md5;

use anyhow::Result;
use data::models::file::FileType;
use zip::ZipArchive;
use std::{io::BufReader, fs::File, path::Path};
use bitflags::bitflags;

use crate::data::{self, models::{file::DataFile, game::Game, set::GameSet}};

use self::{md5::MD5Hasher, sha1::SHA1Hasher};

bitflags! {
    pub struct FileChecks: u32 {
        const SHA1 = 0b00000001;
        const MD5 = 0b00000010;
        const SIZE = 0b00000100;
        const CRC = 0b00001000;
        const ALL = Self::SHA1.bits | Self::MD5.bits | Self::SIZE.bits | Self::CRC.bits;
    }
}

pub struct FileReader {
    sha1_hasher: SHA1Hasher,
    md5_hasher: MD5Hasher,
}


impl FileReader {
    pub fn new() -> Self {
        Self { 
            sha1_hasher: SHA1Hasher::new(),
            md5_hasher: MD5Hasher::new(),
        } 
    }

    pub fn get_game_set(&mut self, game_file_name: &String, file_checks: FileChecks) -> Result<GameSet> {
        let file_path = Path::new(game_file_name);
        let no_path = file_path.with_extension("");
        let base_file_name = no_path.file_name();

        let game_name = match base_file_name {
            Some(str_name) => {
                str_name.to_str().unwrap_or_default()
            }
            None => { "" }
        };

        let game = Game {
            name: game_name.to_string(),
            clone_of: None,
            rom_of: None,
            source_file: None,
            info_description: None,
            info_year: None,
            info_manufacturer: None,
        };

        let use_sha1 = file_checks.contains(FileChecks::SHA1);
        let use_md5 = file_checks.contains(FileChecks::MD5);
        let use_crc = file_checks.contains(FileChecks::CRC);
        let use_size = file_checks.contains(FileChecks::SIZE);

        let mut roms = vec![];
        let file = File::open(&file_path)?;
        let reader = BufReader::new(file);
        let mut archive = ZipArchive::new(reader)?;
        for i in 0..archive.len() {
            let mut f = archive.by_index(i)?;
            let mut writer = vec![];
            std::io::copy(&mut f, &mut writer)?;

            let sha1 =  if use_sha1 { Some(self.sha1_hasher.get_hash(&writer)) } else { None };
            let md5 =  if use_md5 { Some(self.md5_hasher.get_hash(&writer)) } else { None };
            let size = if use_size { Some(f.size() as u32) } else { None };
            let crc = if use_crc { Some(format!("{:x}", f.crc32())) } else { None };

            let rom = DataFile {
                file_type: FileType::Rom,
                name: Some(f.name().to_string()),
                sha1,
                md5,
                crc,
                size,
                status: None,
            };
            
            roms.push(rom);
        }

        let game_set = GameSet::new(game, roms, vec![], vec![]);

        Ok(game_set)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gets_zip_info() -> Result<()> {
        let mut file_reader: FileReader = FileReader::new();
        let game_set = file_reader.get_game_set(&"testdata/split/game1.zip".to_string(), FileChecks::ALL)?;
        println!("Game Set: {}", game_set);

        assert!(game_set.game.name == "game1");
        assert!(game_set.roms.len() == 4);
        assert!(game_set.roms.into_iter().filter(|rom| {
            rom.name == Some("rom1.trom".to_string())
            && rom.sha1 == Some("8bb3a81b9fa2de5163f0ffc634a998c455bcca25".to_string())
            && rom.md5 == Some("aa818fc7769cdd51149f794b0d4fbec9".to_string())
            && rom.crc == Some("1d460eee".to_string())
            && rom.size == Some(2048)
        }).collect::<Vec<_>>().len() == 1);

        Ok(())
    }
}