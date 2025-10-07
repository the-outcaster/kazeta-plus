use crate::{PathBuf, fs, HashSet, get_user_data_dir};

// Scans the 'sfx/' directory for sound pack folders.
pub fn find_sound_packs() -> Vec<String> {
    let mut packs = HashSet::new();
    packs.insert("Default".to_string()); // "Default" is always an option

    // 1. Scan the system directory relative to the BIOS
    let system_sfx_dir = std::path::Path::new("../sfx");
    if let Ok(entries) = fs::read_dir(system_sfx_dir) {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                packs.insert(entry.file_name().to_string_lossy().into_owned());
            }
        }
    }

    // 2. Scan the user's data directory
    if let Some(user_sfx_dir) = get_user_data_dir().map(|d| d.join("sfx")) {
        if let Ok(entries) = fs::read_dir(user_sfx_dir) {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    packs.insert(entry.file_name().to_string_lossy().into_owned());
                }
            }
        }
    }

    // 3. Convert the set back to a sorted list for the UI
    let mut sorted_packs: Vec<String> = packs.into_iter().collect();
    sorted_packs.sort();
    sorted_packs
}

/// Scans a directory and returns a sorted list of paths for files with given extensions.
pub fn find_asset_files(dir_path: &str, extensions: &[&str]) -> Vec<PathBuf> {
    if let Ok(entries) = fs::read_dir(dir_path) {
        let mut files: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|path| {
            path.is_file() &&
            path.extension()
            .and_then(|s| s.to_str())
            .map_or(false, |ext| extensions.contains(&ext))
        })
        .collect();
        files.sort();
        return files;
    }
    vec![]
}
