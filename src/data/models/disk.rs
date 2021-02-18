use std::fmt::{self, Display};

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct GameDiskInfo {
    pub sha1: Option<String>,
    pub region: Option<String>,
    pub status: Option<String>
}

impl GameDiskInfo {
    pub fn new() -> Self { Self { sha1: None, region: None, status: None } }
}

impl Display for GameDiskInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut file_data = vec![];

        if let Some(sha1) = &self.sha1 {
            file_data.push(format!("sha1: {}", sha1))
        }
        if let Some(region) = &self.region {
            file_data.push(format!("region: {}", region))
        }
        if let Some(status) = &self.status {
            file_data.push(format!("status: {}", status))
        }

        write!(f, "{}", file_data.join(", "))
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct GameDisk {
    pub name: String,
    pub info: GameDiskInfo,
}

impl GameDisk {
    pub fn new<S>(name: S) -> Self where S: Into<String> { Self { name: name.into(), info: GameDiskInfo::new() } }
}

impl Display for GameDisk {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[Disk] {}: {}", self.name, self.info)
    }
}
