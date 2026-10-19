use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

// Case insensitive view of the game data folder and the save folder
pub struct Files {
    data: HashMap<String, PathBuf>,
    saves: PathBuf,
}

impl Files {
    pub fn new(data_dir: &Path, saves_dir: &Path) -> std::io::Result<Self> {
        let mut data = HashMap::new();
        for entry in fs::read_dir(data_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                data.insert(entry.file_name().to_string_lossy().to_lowercase(), entry.path());
            }
        }
        fs::create_dir_all(saves_dir)?;
        Ok(Self { data, saves: saves_dir.to_path_buf() })
    }

    pub fn read_data(&self, name: &str) -> Option<Vec<u8>> {
        let key = name.rsplit(['\\', '/']).next().unwrap_or(name).to_lowercase();
        fs::read(self.data.get(&key)?).ok()
    }

    fn save_path(&self, name: &str) -> PathBuf {
        self.saves.join(name)
    }

    // Save names in directory order, matching how the original enumerated *.crn
    pub fn list_saves(&self) -> Vec<String> {
        let Ok(rd) = fs::read_dir(&self.saves) else { return Vec::new() };
        let mut names: Vec<String> = rd
            .flatten()
            .filter_map(|e| {
                let n = e.file_name().to_string_lossy().into_owned();
                n.to_lowercase().ends_with(".crn").then_some(n)
            })
            .collect();
        names.sort();
        names
    }

    pub fn read_save(&self, name: &str) -> Option<Vec<u8>> {
        fs::read(self.save_path(name)).ok()
    }

    pub fn write_save(&self, name: &str, data: &[u8]) -> bool {
        fs::write(self.save_path(name), data).is_ok()
    }

    pub fn save_exists(&self, name: &str) -> bool {
        self.save_path(name).is_file()
    }

    pub fn delete_save(&self, name: &str) {
        let _ = fs::remove_file(self.save_path(name));
    }
}
