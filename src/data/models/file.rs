use std::{cmp::Ordering, fmt::{self, Display}};
use std::cmp::Ord;

#[derive(Debug, PartialEq, Eq, Hash)]
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

#[derive(Debug, Eq, Hash)]
pub struct DataFile {
    pub file_type: FileType,
    pub name: Option<String>,
    pub sha1: Option<String>,
    pub md5: Option<String>,
    pub crc: Option<String>,
    pub size: Option<u32>,
    pub status: Option<String>,
}

impl PartialEq for DataFile {
    fn eq(&self, other: &Self) -> bool {
        let name = match  (self.name.as_ref(), other.name.as_ref()) {
            (Some(self_name), Some(other_name)) => {
                self_name.eq(other_name)
            },
            (None, None) => true ,
            _ => false
        };
        
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
        let mut self_name = self.name.to_owned().unwrap_or_else(|| {"".to_string()});
        let mut other_name = other.name.to_owned().unwrap_or_else(|| {"".to_string()});

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
    pub fn new(file_type: FileType) -> Self { Self { file_type, name: None, sha1: None, md5: None, crc: None, size: None, status: None } }
}

impl Display for DataFile {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut rom_data = vec![];
        if let Some(name) = &self.name {
            rom_data.push(format!("name: {}", name));
        }
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
