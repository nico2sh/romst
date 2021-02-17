use std::fmt::{self, Display};

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct GameDisk {
    pub name: String,
    pub sha1: Option<String>,
    pub region: Option<String>,
    pub status: Option<String>
}

impl GameDisk {
    pub fn new<S>(name: S) -> Self where S: Into<String> { Self { name: name.into(), sha1: None, region: None, status: None } }
}

impl Display for GameDisk {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut file_data = vec![];

        if let Some(sha1) = &self.sha1 {
            file_data.push(format!("sha1: {}", sha1))
        }
        if let Some(status) = &self.status {
            file_data.push(format!("status: {}", status))
        }

        write!(f, "[Disk] {}: {}", self.name, file_data.join(", "))
    }
}
