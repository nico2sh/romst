mod sha1;
mod md5;

use anyhow::Result;
use data::models::file::FileType;
use zip::{ZipArchive, result::ZipError};
use std::{fs::File, io::BufReader, path::Path};
use bitflags::bitflags;

use crate::{data::{self, models::{file::{DataFile, DataFileInfo}, game::Game, set::GameSet}}, error::RomstIOError};

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

    pub fn get_game_set(&mut self, file_path: &impl AsRef<Path>, file_checks: FileChecks) -> Result<GameSet, RomstIOError> {
        let no_path = Path::new(file_path.as_ref()).with_extension("");
        let base_file_name = no_path.file_name();

        let game_name = match base_file_name {
            Some(str_name) => {
                str_name.to_str().unwrap_or_default()
            }
            None => { "" }
        };

        let game = Game::new(game_name.to_string());

        let use_sha1 = file_checks.contains(FileChecks::SHA1);
        let use_md5 = file_checks.contains(FileChecks::MD5);
        let use_crc = file_checks.contains(FileChecks::CRC);
        let use_size = file_checks.contains(FileChecks::SIZE);

        let mut roms = vec![];
        let file = File::open(&file_path)?;
        let reader = BufReader::new(file);

        match ZipArchive::new(reader) {
            Ok(mut archive) => {
                for i in 0..archive.len() {
                    let mut f = archive.by_index(i).map_err(|err| { RomstIOError::Io{ source: err.into() } })?;
                    let mut writer = vec![];
                    std::io::copy(&mut f, &mut writer)?;

                    let sha1 =  if use_sha1 { Some(self.sha1_hasher.get_hash(&writer)) } else { None };
                    let md5 =  if use_md5 { Some(self.md5_hasher.get_hash(&writer)) } else { None };
                    let size = if use_size { Some(f.size() as u32) } else { None };
                    let crc = if use_crc { Some(format!("{:x}", f.crc32())) } else { None };

                    let rom = DataFile {
                        name: f.name().to_string(),
                        info: DataFileInfo {
                            file_type: FileType::Rom,
                            sha1,
                            md5,
                            crc,
                            size,
                            status: None,
                        }
                    };
                    
                    roms.push(rom);
                }
            },
            Err(ZipError::InvalidArchive(e)) => {
                let file_name = file_path.as_ref().to_path_buf().into_os_string().into_string().unwrap_or_else(|ref osstring| {
                    osstring.to_string_lossy().to_string()
                });
                return Err(RomstIOError::NotValidFileError(file_name, FileType::Rom))
            },
            Err(ZipError::FileNotFound) => {
                let file_name = file_path.as_ref().to_path_buf().into_os_string().into_string().unwrap_or_else(|ref osstring| {
                    osstring.to_string_lossy().to_string()
                });
                return Err(RomstIOError::FileNotFound(file_name))
            },
            Err(e) => { return Err(RomstIOError::Io{ source: e.into() }) }
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
        let file_path = Path::new("testdata").join("split").join("game1.zip");
        let game_set = file_reader.get_game_set(&file_path, FileChecks::ALL)?;
        println!("Game Set: {}", game_set);

        assert!(game_set.game.name == "game1");
        assert!(game_set.roms.len() == 4);
        assert!(game_set.roms.into_iter().filter(|rom| {
            rom.name == "rom1.trom".to_string()
            && rom.info.sha1 == Some("8bb3a81b9fa2de5163f0ffc634a998c455bcca25".to_string())
            && rom.info.md5 == Some("aa818fc7769cdd51149f794b0d4fbec9".to_string())
            && rom.info.crc == Some("1d460eee".to_string())
            && rom.info.size == Some(2048)
        }).collect::<Vec<_>>().len() == 1);

        Ok(())
    }
}