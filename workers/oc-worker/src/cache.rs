use std::env;
use std::fs;
use std::path::PathBuf;

fn get_cache_dir() -> PathBuf {
    let base_dir = match env::home_dir() {
        Some(path) => path.join(".cache"),
        None => env::temp_dir(),
    };

    base_dir.join("oc-worker")
}

fn get_cache_file_path() -> PathBuf {
    let cache_dir = get_cache_dir();
    if !cache_dir.exists() {
        fs::create_dir_all(&cache_dir).expect("Failed to create cache directory");
    }
    cache_dir.join("last_updated.txt")
}

pub fn save_last_updated(last_updated: i64) {
    let cache_file_path = get_cache_file_path();
    fs::write(&cache_file_path, last_updated.to_string()).expect("Failed to write to cache file");
}

pub fn get_last_updated() -> Option<i64> {
    let cache_file_path = get_cache_file_path();
    let last_updated = match fs::read_to_string(&cache_file_path) {
        Ok(content) => content.trim().parse::<i64>().ok(),
        Err(_) => {
            sheen::warn!(
                "Cache file not found or unreadable",
                cache_file_path = cache_file_path
            );
            None
        }
    };

    last_updated
}
