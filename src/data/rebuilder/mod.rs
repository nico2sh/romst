use super::models::file::DataFile;

struct GameSetRebuildList {
    game_sets: Vec<GameSetRebuild>
}

struct GameSetRebuild {
    name: String,
    have_files: Vec<HaveFiles>,
    missing_files: Vec<DataFile>
}

struct HaveFiles {
    file_name: String,
    from_file_set: String,
    rename_to: Option<String>
}