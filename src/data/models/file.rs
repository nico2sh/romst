use std::{cmp::Ordering, fmt::{self, Display}, hash::Hash};
use std::cmp::Ord;
use serde::{Deserialize, Serialize};

use filesystem::FileChecks;
use anyhow::Result;

use crate::{error::RomstError, err, filesystem};

#[derive(Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FileType {
    Rom,
    Disk,
    Sample
}

impl Display for FileType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FileType::Rom => write!(f, "ROM"),
            FileType::Disk => write!(f, "Disk"),
            FileType::Sample => write!(f, "Sample"),
        }
    }
}

#[derive(Debug, Eq, Serialize, Deserialize)]
pub struct DataFile {
    pub file_type: FileType,
    pub name: String,
    pub sha1: Option<String>,
    pub md5: Option<String>,
    pub crc: Option<String>,
    pub size: Option<u32>,
    pub status: Option<String>,
}

impl Hash for DataFile {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.sha1.hash(state);
        self.md5.hash(state);
        self.crc.hash(state);
    }
}

impl PartialEq for DataFile {
    fn eq(&self, other: &Self) -> bool {
        let name = self.name.eq(&other.name);
        
        // We are good just with the sha1
        match  (self.sha1.as_ref(), other.sha1.as_ref()) {
            (Some(self_sha1), Some(other_sha1)) => {
                return name && self_sha1.eq(other_sha1);
            },
            _ => { }
        }
        // MD5 in case of emergency
        match  (self.md5.as_ref(), other.md5.as_ref()) {
            (Some(self_md5), Some(other_md5)) => {
                return name && self_md5.eq(other_md5);
            },
            _ => { }
        }
        // uh-ooohh
        match  (self.crc.as_ref(), other.crc.as_ref()) {
            (Some(self_crc), Some(other_crc)) => {
                return name && self_crc.eq(other_crc);
            },
            _ => { }
        }

        return name;
    }
}

impl Ord for DataFile {
    fn cmp(&self, other: &Self) -> Ordering {
        // we use the name as prefix to sort
        let mut self_name = self.name.to_owned();
        let mut other_name = other.name.to_owned();

        // We are good just with the sha1
        match  (self.sha1.as_ref(), other.sha1.as_ref()) {
            (Some(self_sha1), Some(other_sha1)) => {
                return self_name.push_str(self_sha1).cmp(&other_name.push_str(other_sha1));
            },
            _ => { }
        }

        // MD5 in case of emergency
        match  (self.md5.as_ref(), other.md5.as_ref()) {
            (Some(self_md5), Some(other_md5)) => {
                return self_name.push_str(self_md5).cmp(&other_name.push_str(other_md5));
            },
            _ => { }
        }

        // uh-ooohh
        match  (self.crc.as_ref(), other.crc.as_ref()) {
            (Some(self_crc), Some(other_crc)) => {
                return self_name.push_str(self_crc).cmp(&other_name.push_str(other_crc));
            },
            _ => { }
        }

        return self_name.cmp(&other_name);
    }
}

impl PartialOrd for DataFile {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl DataFile {
    pub fn new(file_type: FileType, name: String) -> Self { Self { file_type, name, sha1: None, md5: None, crc: None, size: None, status: None } }

    /// Compares two files with the requested info, if the info is not available in either file, the comparation is ignored
    pub fn deep_compare(&self, other: &Self, file_checks: FileChecks, include_name: bool) -> Result<bool> {
        let mut compared = false;
        let mut result = if include_name {
            self.name.eq(&other.name)
        } else {
            true
        };
        
        if file_checks.contains(FileChecks::SHA1) {
            result = result && match (self.sha1.as_ref(), other.sha1.as_ref()) {
                (Some(self_sha1), Some(other_sha1)) => {
                    compared = true;
                    self_sha1.eq(other_sha1)
                },
                _ => { true }
            };
        }

        if file_checks.contains(FileChecks::MD5) {
            result = result && match (self.md5.as_ref(), other.md5.as_ref()) {
                (Some(self_md5), Some(other_md5)) => {
                    compared = true;
                    self_md5.eq(other_md5)
                },
                _ => { true }
            }
        }

        if file_checks.contains(FileChecks::CRC) {
            result = result && match (self.crc.as_ref(), other.crc.as_ref()) {
                (Some(self_crc), Some(other_crc)) => {
                    compared = true;
                    self_crc.eq(other_crc)
                },
                _ => { true }
            }
        }

        if file_checks.contains(FileChecks::SIZE) {
            result = result && match (self.size.as_ref(), other.size.as_ref()) {
                (Some(self_size), Some(other_size)) => {
                    compared = true;
                    self_size.eq(other_size)
                },
                _ => { true }
            }
        }

        if compared {
            Ok(result)
        } else {
            err!(RomstError::GenericError {
                message: format!("Can't compare, not enough info:\n{}\n{}", self, other)
            })
        }
    }
}

impl Display for DataFile {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut rom_data = vec![];
        rom_data.push(format!("name: {}", self.name));

        if let Some(sha1) = &self.sha1 {
            rom_data.push(format!("sha1: {}", sha1))
        }
        if let Some(md5) = &self.md5 {
            rom_data.push(format!("md5: {}", md5))
        }
        if let Some(crc) = &self.crc {
            rom_data.push(format!("crc: {}", crc))
        }
        if let Some(size) = &self.size {
            rom_data.push(format!("size: {}", size))
        }
        if let Some(status) = &self.status {
            rom_data.push(format!("status: {}", status))
        }

        write!(f, "{}: ({})", self.file_type, rom_data.join(", "))
    }
}
