mod sha1;
mod md5;

use anyhow::Result;
use zip::ZipArchive;
use std::{io::BufReader, fs::File, path::Path};

use self::{md5::MD5Hasher, sha1::SHA1Hasher};

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

    pub fn get_roms(&mut self, game_name: String) -> Result<()> {
        println!("Reading file {}", game_name);

        let file_path = Path::new(&game_name);
        let file = File::open(&file_path)?;

        let reader = BufReader::new(file);

        let mut archive = ZipArchive::new(reader)?;
        for i in 0..archive.len() {
            let mut f = archive.by_index(i)?;
            let mut writer = vec![];
            std::io::copy(&mut f, &mut writer)?;

            let hash_sha1 =  self.sha1_hasher.get_hash(&writer);
            let hash_md5 =  self.md5_hasher.get_hash(&writer);
            let crc_string = format!("{:x}", f.crc32());
            
            println!("{} - SHA1 {} - MD5 {} - CRC {} - size {}", f.name(), hash_sha1, hash_md5, crc_string, f.size());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gets_zip_info() -> Result<()> {
        let mut file_reader: FileReader = FileReader::new();
        file_reader.get_roms("testdata/circus.zip".to_string())?;

        assert!(true);

        Ok(())
    }
}