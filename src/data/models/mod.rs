use std::path::Path;

pub mod set;
pub mod game;
pub mod file;

pub fn does_file_belong_to_set<S>(file: S, set: S) -> bool where S: Into<String> {
    let f = file.into();
    let set_name: String = set.into();
    let file_path = Path::new(&f);
    if is_extension_for_file_set(&file_path) {
        if let Some(file_name) = file_path.file_stem() {
            if file_name.eq(set_name.as_str()) {
                return true;
            }
        };
    };

    false
}

fn is_extension_for_file_set(file: &impl AsRef<Path>) -> bool {
    if let Some(extension) = file.as_ref().extension() {
        return extension.eq("zip");
    }

    false
}

#[cfg(test)]
mod tests {
    use super::does_file_belong_to_set;

    #[test]
    pub fn should_identify_a_set() {
        assert!(does_file_belong_to_set("set.zip", "set"))
    }

    #[test]
    pub fn should_identify_a_non_set() {
        assert!(!does_file_belong_to_set("file.zip", "set"))
    }
}