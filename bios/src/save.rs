use std::path::Path;
use std::fs;
use std::io::{self, BufRead, Write, Read};
use sysinfo::Disks;
use tar::{Builder, Archive};
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::Arc;
use walkdir;

pub fn get_save_dir_from_drive_name(drive_name: &str) -> String {
    let base_dir = dirs::home_dir().unwrap().join(".local/share/kazeta");
    if drive_name == "internal" || drive_name.is_empty() {
        let save_dir = base_dir.join("saves/default");
        if !save_dir.exists() {
            fs::create_dir_all(&save_dir).unwrap_or_else(|e| {
                eprintln!("Failed to create save directory: {}", e);
            });
        }
        save_dir.to_string_lossy().into_owned()
    } else {
        let base_ext = if Path::new("/media").read_dir().map(|mut d| d.next().is_none()).unwrap_or(true) {
            if Path::new(&format!("/run/media/{}", whoami::username())).exists() {
                format!("/run/media/{}", whoami::username())
            } else {
                "/run/media".to_string()
            }
        } else {
            "/media".to_string()
        };

        let save_dir = Path::new(&base_ext).join(drive_name).join("kazeta/saves");
        if !save_dir.exists() {
            fs::create_dir_all(&save_dir).unwrap_or_else(|e| {
                eprintln!("Failed to create save directory: {}", e);
            });
        }
        save_dir.to_string_lossy().into_owned()
    }
}

pub fn get_cache_dir_from_drive_name(drive_name: &str) -> String {
    let base_dir = dirs::home_dir().unwrap().join(".local/share/kazeta");
    if drive_name == "internal" || drive_name.is_empty() {
        let cache_dir = base_dir.join("cache");
        if !cache_dir.exists() {
            fs::create_dir_all(&cache_dir).unwrap_or_else(|e| {
                eprintln!("Failed to create cache directory: {}", e);
            });
        }
        cache_dir.to_string_lossy().into_owned()
    } else {
        let base_ext = if Path::new("/media").read_dir().map(|mut d| d.next().is_none()).unwrap_or(true) {
            if Path::new(&format!("/run/media/{}", whoami::username())).exists() {
                format!("/run/media/{}", whoami::username())
            } else {
                "/run/media".to_string()
            }
        } else {
            "/media".to_string()
        };

        let cache_dir = Path::new(&base_ext).join(drive_name).join("kazeta/cache");
        if !cache_dir.exists() {
            fs::create_dir_all(&cache_dir).unwrap_or_else(|e| {
                eprintln!("Failed to create cache directory: {}", e);
            });
        }
        cache_dir.to_string_lossy().into_owned()
    }
}

fn get_attribute(info_file: &Path, attribute: &str) -> io::Result<String> {
    let file = fs::File::open(info_file)?;
    let reader = io::BufReader::new(file);

    for line in reader.lines() {
        let line = line?;
        if line.starts_with(&format!("{}=", attribute)) {
            return Ok(line[attribute.len() + 1..].to_string());
        }
    }

    Ok(String::new())
}

pub fn list_devices() -> io::Result<Vec<(String, u32)>> {
    let mut devices = Vec::new();
    let disks = Disks::new_with_refreshed_list();

    // Add internal drive
    let base_dir = dirs::home_dir().unwrap().join(".local/share/kazeta");
    let base_dir_str = base_dir.to_str().unwrap();

    // Find the disk that contains our base directory
    let internal_disk = disks.iter()
        .find(|disk| {
            let mount_point = disk.mount_point().to_str().unwrap();
            base_dir_str.starts_with(mount_point)
        })
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Could not find internal disk"))?;

    let free_space = (internal_disk.available_space() / 1024 / 1024) as u32; // Convert to MB
    devices.push(("internal".to_string(), free_space));

    // Add external drives
    let base_ext = if Path::new("/media").read_dir().map(|mut d| d.next().is_none()).unwrap_or(true) {
        if Path::new(&format!("/run/media/{}", whoami::username())).exists() {
            format!("/run/media/{}", whoami::username())
        } else {
            "/run/media".to_string()
        }
    } else {
        "/media".to_string()
    };

    // Find all disks mounted under the external base directory
    for disk in disks.iter() {
        let mount_point = disk.mount_point().to_str().unwrap();
        if mount_point.starts_with(&base_ext) {
            let name = mount_point.split('/').last().unwrap().to_string();
            let free_space = (disk.available_space() / 1024 / 1024) as u32; // Convert to MB
            devices.push((name, free_space));
        }
    }

    Ok(devices)
}

pub fn get_save_details(drive_name: &str) -> io::Result<Vec<(String, String, String, u16)>> {
    let save_dir = get_save_dir_from_drive_name(drive_name);
    let cache_dir = get_cache_dir_from_drive_name(drive_name);
    eprintln!("Getting save details from directory: {}", save_dir);
    let mut details = Vec::new();

    for entry in fs::read_dir(save_dir)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = path.file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid filename"))?;

        // Remove .kzs extension if present
        let cart_id = if file_name.ends_with(".kzs") {
            &file_name[..file_name.len() - 4]
        } else {
            file_name
        };

        let metadata_path = Path::new(&cache_dir).join(cart_id).join("metadata.kzi");
        let name = get_attribute(&metadata_path, "Name").unwrap_or_else(|e| {
            eprintln!("Failed to read metadata for {}: {}", cart_id, e);
            String::new()
        });
        let icon = format!("{}/{}/icon.png", cache_dir, cart_id);

        let size = if path.extension().and_then(|e| e.to_str()) == Some("kzs") {
            // For .kzs files, get the file size
            let metadata = fs::metadata(&path)?;
            (metadata.len() / 1024 / 1024) as u16 // Convert to MB
        } else {
            // For directories, get the directory size excluding .cache
            let mut total_size = 0;
            for entry in fs::read_dir(&path)? {
                let entry = entry?;
                let entry_path = entry.path();
                if entry_path.file_name().and_then(|n| n.to_str()) != Some(".cache") {
                    if entry_path.is_file() {
                        total_size += fs::metadata(&entry_path)?.len();
                    }
                }
            }
            (total_size / 1024 / 1024) as u16 // Convert to MB
        };

        details.push((cart_id.to_string(), name, icon, size));
    }

    eprintln!("Found {} save details", details.len());
    Ok(details)
}

pub fn delete_save(cart_id: &str, from_drive: &str) -> Result<(), String> {
    let from_dir = get_save_dir_from_drive_name(from_drive);
    let from_cache = get_cache_dir_from_drive_name(from_drive);

    // Check if save exists
    let save_path = Path::new(&from_dir).join(cart_id);
    let save_path_kzs = Path::new(&from_dir).join(format!("{}.kzs", cart_id));
    if !save_path.exists() && !save_path_kzs.exists() {
        return Err(format!("Save file for {} does not exist on '{}' drive", cart_id, from_drive));
    }

    // Delete save file
    if from_drive == "internal" {
        fs::remove_dir_all(save_path).map_err(|e| e.to_string())?;
    } else {
        fs::remove_file(save_path_kzs).map_err(|e| e.to_string())?;
    }

    // Delete cache
    let cache_path = Path::new(&from_cache).join(cart_id);
    if cache_path.exists() {
        fs::remove_dir_all(cache_path).map_err(|e| e.to_string())?;
    }

    Ok(())
}

pub fn copy_save(cart_id: &str, from_drive: &str, to_drive: &str, progress: Arc<AtomicU16>) -> Result<(), String> {
    let from_dir = get_save_dir_from_drive_name(from_drive);
    let to_dir = get_save_dir_from_drive_name(to_drive);
    let from_cache = get_cache_dir_from_drive_name(from_drive);
    let to_cache = get_cache_dir_from_drive_name(to_drive);

    if from_drive == to_drive {
        return Err("Cannot copy to same location".to_string());
    }

    // Check if source save exists
    let from_path = Path::new(&from_dir).join(cart_id);
    let from_path_kzs = Path::new(&from_dir).join(format!("{}.kzs", cart_id));
    if !from_path.exists() && !from_path_kzs.exists() {
        return Err(format!("Save file for {} does not exist on '{}' drive", cart_id, from_drive));
    }

    // Check if destination save already exists
    let to_path = Path::new(&to_dir).join(cart_id);
    let to_path_kzs = Path::new(&to_dir).join(format!("{}.kzs", cart_id));
    if to_path.exists() || to_path_kzs.exists() {
        return Err(format!("Save file for {} already exists on '{}'", cart_id, to_drive));
    }

    // Create destination directories
    fs::create_dir_all(&to_dir).map_err(|e| e.to_string())?;
    fs::create_dir_all(&to_cache).map_err(|e| e.to_string())?;

    // Copy save data
    if from_drive == "internal" {
        // Internal to external: create tar archive
        let file = fs::File::create(&to_path_kzs).map_err(|e| e.to_string())?;
        let mut builder = Builder::new(file);

        // Calculate total size for progress reporting
        let mut total_size = 0;
        for entry in walkdir::WalkDir::new(&from_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                let path = e.path();
                path.is_file() && path.file_name().and_then(|n| n.to_str()) != Some(".cache")
            }) {
            total_size += entry.metadata().map_err(|e| e.to_string())?.len();
        }

        eprintln!("Total size to archive: {} bytes", total_size);

        // Add the entire directory to the archive, excluding .cache
        let mut current_size = 0;
        for entry in walkdir::WalkDir::new(&from_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                let path = e.path();
                path.is_file() && path.file_name().and_then(|n| n.to_str()) != Some(".cache")
            }) {
            let path = entry.path();
            let name = path.strip_prefix(&from_path).unwrap().to_str().unwrap();
            eprintln!("Adding file to archive: {} ({} bytes)", name, entry.metadata().unwrap().len());

            let mut file = fs::File::open(path).map_err(|e| e.to_string())?;
            builder.append_file(name, &mut file).map_err(|e| e.to_string())?;

            current_size += entry.metadata().map_err(|e| e.to_string())?.len();
            progress.store((current_size * 100 / total_size) as u16, Ordering::SeqCst);
        }

        eprintln!("Finished creating archive, final size: {} bytes", current_size);
        builder.finish().map_err(|e| e.to_string())?;
    } else if to_drive == "internal" {
        // External to internal: extract tar archive
        fs::create_dir_all(&to_path).map_err(|e| e.to_string())?;

        let file = fs::File::open(&from_path_kzs).map_err(|e| e.to_string())?;
        let file_size = file.metadata().map_err(|e| e.to_string())?.len();
        let mut archive = Archive::new(file);

        let mut current_size = 0;
        for entry in archive.entries().map_err(|e| e.to_string())? {
            let mut entry = entry.map_err(|e| e.to_string())?;
            entry.unpack_in(&to_path).map_err(|e| e.to_string())?;
            current_size += entry.header().size().unwrap_or(0);
            progress.store((current_size * 100 / file_size) as u16, Ordering::SeqCst);
        }
    } else {
        // External to external: direct copy with progress
        let file_size = fs::metadata(&from_path_kzs).map_err(|e| e.to_string())?.len();
        let mut source = fs::File::open(&from_path_kzs).map_err(|e| e.to_string())?;
        let mut dest = fs::File::create(&to_path_kzs).map_err(|e| e.to_string())?;

        let mut buffer = [0; 8192];
        let mut current_size = 0;
        loop {
            let bytes_read = source.read(&mut buffer).map_err(|e| e.to_string())?;
            if bytes_read == 0 {
                break;
            }
            dest.write_all(&buffer[..bytes_read]).map_err(|e| e.to_string())?;
            current_size += bytes_read as u64;
            progress.store((current_size * 100 / file_size) as u16, Ordering::SeqCst);
        }
    }

    // Copy cache files
    let to_cache_path = Path::new(&to_cache).join(cart_id);
    fs::remove_dir_all(&to_cache_path).ok(); // Ignore errors if directory doesn't exist
    fs::create_dir_all(&to_cache_path).map_err(|e| e.to_string())?;

    // Copy metadata.kzi if it exists
    let from_metadata = Path::new(&from_cache).join(cart_id).join("metadata.kzi");
    let to_metadata = to_cache_path.join("metadata.kzi");
    if from_metadata.exists() {
        fs::copy(&from_metadata, &to_metadata).map_err(|e| e.to_string())?;
    }

    // Copy icon.png if it exists
    let from_icon = Path::new(&from_cache).join(cart_id).join("icon.png");
    let to_icon = to_cache_path.join("icon.png");
    if from_icon.exists() {
        fs::copy(&from_icon, &to_icon).map_err(|e| e.to_string())?;
    }

    Ok(())
}