use std::fmt;

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum FileType {
    Rom,
    Disk,
    Sample
}

impl fmt::Display for FileType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FileType::Rom => write!(f, "ROM"),
            FileType::Disk => write!(f, "Disk"),
            FileType::Sample => write!(f, "Sample"),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct DataFile {
    pub file_type: FileType,
    pub name: Option<String>,
    pub sha1: Option<String>,
    pub md5: Option<String>,
    pub crc: Option<String>,
    pub size: Option<u32>,
    pub status: Option<String>,
}

impl DataFile {
    pub fn new(file_type: FileType) -> Self { Self { file_type, name: None, sha1: None, md5: None, crc: None, size: None, status: None } }
}

impl fmt::Display for DataFile {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut rom_data = vec![];
        match self.name {
            Some(ref name) => rom_data.push(format!("name: {}", name)),
            _ => (),
        }
        match self.sha1 {
            Some(ref sha1) => rom_data.push(format!("sha1: {}", sha1)),
            _ => (),
        }
        match self.md5 {
            Some(ref md5) => rom_data.push(format!("md5: {}", md5)),
            _ => (),
        }
        match self.crc {
            Some(ref crc) => rom_data.push(format!("crc: {}", crc)),
            _ => (),
        }
        match self.size {
            Some(ref size) => rom_data.push(format!("size: {}", size)),
            _ => (),
        }
        match self.status {
            Some(ref status) => rom_data.push(format!("status: {}", status)),
            _ => (),
        }

        write!(f, "File Type: {} ({})", self.file_type, rom_data.join(", "))
    }
}
