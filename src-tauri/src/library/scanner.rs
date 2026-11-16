use std::fs;
use std::path::Path;
use anyhow::Result;
use tracing::info;

use crate::db::Database;
use crate::library::ingest::ingest_audio_asset;

pub struct LibraryScanner;

impl LibraryScanner {
    pub fn scan_directory<P: AsRef<Path>>(dir: P, db: &Database) -> Result<usize> {
        let mut count = 0;
        let mut paths_to_scan = vec![dir.as_ref().to_path_buf()];

        let supported_extensions = ["mp3", "wav", "flac", "m4a", "aac", "ogg"];

        while let Some(current_dir) = paths_to_scan.pop() {
            if let Ok(entries) = fs::read_dir(&current_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        paths_to_scan.push(path);
                    } else if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                        if supported_extensions.contains(&ext.to_lowercase().as_str()) {
                            if let Ok(_res) = ingest_audio_asset(&path, "local_filesystem", None, false, db) {
                                count += 1;
                            }
                        }
                    }
                }
            }
        }

        info!("Scanned and indexed {} audio files into library", count);
        Ok(count)
    }
}
