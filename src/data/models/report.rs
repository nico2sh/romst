use std::fmt::{self, Display};

#[derive(Debug)]
pub struct Report {
    pub name: String,
    pub roms_have: Vec<String>,
    pub roms_missing: Vec<String>,
}

impl Report {
    pub fn new(name: String, roms_have: Vec<String>, roms_missing: Vec<String>) -> Self { Self { name, roms_have, roms_missing } }

    pub fn add_having(&mut self, rom: String) {
        self.roms_have.push(rom);
    }

    pub fn add_missing(&mut self, rom: String) {
        self.roms_missing.push(rom);
    }
}


impl Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut output = format!("{}", self.name);
        if self.roms_have.len() > 0 {
            output.push_str("\nRoms:");
            for have in self.roms_have.as_slice() {
                output.push_str(&format!("\n    - {}", have));
            }
        }

        if self.roms_missing.len() > 0 {
            output.push_str("\nMissing:");
            for missing in self.roms_missing.as_slice() {
                output.push_str(&format!("\n    - {}", missing));
            }
        }

        write!(f, "{}", output)
    }
}