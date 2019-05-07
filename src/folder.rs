use std::path::{Path, PathBuf};

use filesize::file_real_size;
use ignore::WalkBuilder;
use serde_derive::Serialize;
use serde_json;

#[derive(Debug, Clone, Serialize)]
pub struct FileInfo {
    pub path: PathBuf,
    pub logical_size: u64,
    pub physical_size: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct FolderInfo {
    pub path: PathBuf,
    pub logical_size: u64,
    pub physical_size: u64,
    pub compressable: Vec<FileInfo>,
    pub compressed: Vec<FileInfo>,
    pub skipped: Vec<FileInfo>,
}

impl FolderInfo {
    pub fn evaluate<P: AsRef<Path>>(path: P) -> Self {
        let mut ds = Self {
            path: path.as_ref().to_path_buf(),
            logical_size: 0,
            physical_size: 0,
            compressable: vec![],
            compressed: vec![],
            skipped: vec![],
        };

        let skip_exts = vec![
            "7z", "aac", "avi", "bik", "bmp", "br", "bz2", "cab", "dl_", "docx", "flac", "flv",
            "gif", "gz", "jpeg", "jpg", "lz4", "lzma", "lzx", "m2v", "m4v", "mkv", "mp3", "mp4",
            "mpg", "ogg", "onepkg", "png", "pptx", "rar", "vob", "vssx", "vstx", "wma", "wmf",
            "wmv", "xap", "xlsx", "xz", "zip", "zst", "zstd",
        ];

        let walker = WalkBuilder::new(path.as_ref())
            .standard_filters(false)
            .build()
            .filter_map(|e| e.map_err(|e| eprintln!("Error: {:?}", e)).ok())
            .filter_map(|e| e.metadata().map(|md| (e, md)).ok())
            .filter(|(_, md)| md.is_file())
            .filter_map(|(e, md)| file_real_size(e.path()).map(|s| (e, md, s)).ok());

        for (entry, metadata, physical) in walker {
            let logical = metadata.len();
            ds.logical_size += logical;
            ds.physical_size += physical;

            let shortname = entry
                .path()
                .strip_prefix(&path)
                .unwrap_or_else(|_e| entry.path())
                .to_path_buf();
            let extension = entry.path().extension().and_then(std::ffi::OsStr::to_str);

            let fi = FileInfo {
                path: shortname,
                logical_size: logical,
                physical_size: physical,
            };

            if physical < logical {
                ds.compressed.push(fi);
            } else if logical > 4096
                && !extension
                    .map(|ext| skip_exts.iter().any(|ex| ex.eq_ignore_ascii_case(ext)))
                    .unwrap_or_default()
            {
                ds.compressable.push(fi);
            } else {
                ds.skipped.push(fi);
            }
        }

        ds.compressed.sort_by(|a, b| {
            (a.physical_size as f64 / a.logical_size as f64)
                .partial_cmp(&(b.physical_size as f64 / b.logical_size as f64))
                .unwrap()
        });

        ds
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(&self).expect("serde")
    }
}