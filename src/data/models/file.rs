use std::{cmp::Ordering, fmt::{self, Display}, hash::Hash};
use std::cmp::Ord;
use serde::{Deserialize, Serialize};

use filesystem::FileChecks;
use anyhow::Result;

use crate::{error::RomstError, err, filesystem};

#[derive(Debug, Clone, PartialOrd, Ord, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FileType {
    Rom,
    Sample
}

impl Display for FileType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FileType::Rom => write!(f, "ROM"),
            FileType::Sample => write!(f, "Sample"),
        }
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataFile {
    pub name: String,
    pub info: DataFileInfo,
    pub status: Option<String>,
}

impl Ord for DataFile {
    fn cmp(&self, other: &Self) -> Ordering {
        // we use the name as prefix to sort
        let self_name = self.name.to_owned();
        let other_name = other.name.to_owned();

        self_name.cmp(&other_name)
    }
}

impl PartialOrd for DataFile {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Display for DataFile {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut rom_data = vec![];
        rom_data.push(format!("name: {}", self.name));

        write!(f, "{} - {}", self.name, self.info)?;
        if let Some(status) = &self.status {
            write!(f, " ({})", status)?;
        };

        Ok(())
    }
}

impl DataFile {
    pub fn new<S>(name: S, file_info: DataFileInfo) -> Self where S: Into<String> {
        Self {
            name: name.into(),
            info: file_info,
            status: None
        }
    }

    pub fn new_with_status<S>(name: S, file_info: DataFileInfo, status: Option<String>) -> Self where S: Into<String> {
        Self {
            name: name.into(),
            info: file_info,
            status
        }
    }

    /// Compares two files with the requested info, if the info is not available in either file, the comparation is ignored
    pub fn deep_compare(&self, other: &Self, file_checks: FileChecks) -> Result<bool> {
        if !self.name.eq(&other.name) {
            Ok(false)
        } else {
            self.info.deep_compare(&other.info, file_checks)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DataFileInfo {
    pub file_type: FileType,
    pub sha1: Option<String>,
    pub md5: Option<String>,
    pub crc: Option<String>,
    pub size: Option<u32>,
}

impl DataFileInfo {
    pub fn new(file_type: FileType) -> Self {
        Self {
            file_type,
            sha1: None,
            md5: None,
            crc: None,
            size: None,
        }
    }

    pub fn deep_compare(&self, other: &Self, file_checks: FileChecks) -> Result<bool> {
        let mut compared = false;
        let mut result = true;
        
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

/*impl PartialEq for DataFileInfo {
    fn eq(&self, other: &Self) -> bool {

        // We are good just with the sha1
        if let (Some(self_sha1), Some(other_sha1)) = (self.sha1.as_ref(), other.sha1.as_ref()) {
            return self_sha1.eq(other_sha1);
        }
        // MD5 in case of emergency
        if let (Some(self_md5), Some(other_md5)) = (self.md5.as_ref(), other.md5.as_ref()) {
            return self_md5.eq(other_md5);
        }
        // uh-ooohh
        if let (Some(self_crc), Some(other_crc)) = (self.crc.as_ref(), other.crc.as_ref()) {
            return self_crc.eq(other_crc);
        }
        // last resource
        if let (Some(self_size), Some(other_size)) = (self.size.as_ref(), other.size.as_ref()) {
            return self_size.eq(other_size);
        }

        self.file_type.eq(&other.file_type)
    }
}*/

impl Ord for DataFileInfo {
    fn cmp(&self, other: &Self) -> Ordering {
        // We are good just with the sha1
        if let (Some(self_sha1), Some(other_sha1)) = (self.sha1.as_ref(), other.sha1.as_ref()) {
            return self_sha1.cmp(&other_sha1);
        }
        // MD5 in case of emergency
        if let (Some(self_md5), Some(other_md5)) = (self.md5.as_ref(), other.md5.as_ref()) {
            return self_md5.cmp(&other_md5);
        }
        // uh-ooohh
        if let (Some(self_crc), Some(other_crc)) = (self.crc.as_ref(), other.crc.as_ref()) {
            return self_crc.cmp(&other_crc);
        }
        // last resource
        if let (Some(self_size), Some(other_size)) = (self.size.as_ref(), other.size.as_ref()) {
            return self_size.cmp(&other_size);
        }

        self.file_type.cmp(&other.file_type)
    }
}

impl PartialOrd for DataFileInfo {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Display for DataFileInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut rom_data = vec![];

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

        write!(f, "[{}] File Info: {}", self.file_type, rom_data.join(", "))
    }
}