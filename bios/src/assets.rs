use crate::{PathBuf, fs};

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
