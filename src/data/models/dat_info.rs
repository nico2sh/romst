use std::fmt::Display;

use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct DatInfo {
    pub name: String,
    pub description: String,
    pub version: String,
    pub extra_data: Vec<(String, String)>
}

impl DatInfo {
    pub fn new(name: String, description: String, version: String, extra_data: Vec<(String, String)>) -> Self { Self { name, description, version, extra_data } }
}

impl Display for DatInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if !self.name.is_empty() {
            writeln!(f, "Name: {}", self.name)?;
        }
        if !self.description.is_empty() {
            writeln!(f, "Description: {}", self.description)?; 
        }
        if !self.version.is_empty() {
            writeln!(f, "Version: {}", self.description)?; 
        }

        for entry in &self.extra_data {
            writeln!(f, "{}: {}", entry.0, entry.1)?;
        }

        Ok(())
    }
}