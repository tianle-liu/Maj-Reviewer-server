use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

pub fn get_folder_size(folder: &Path) -> u64 {
    let mut size = 0;
    if let Ok(entries) = fs::read_dir(folder) {
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.is_file() {
                    if let Ok(metadata) = fs::metadata(&path) {
                        size += metadata.len();
                    }
                } else if path.is_dir() {
                    size += get_folder_size(&path);
                }
                }
            }
        }
    size
}

pub fn delete_oldest_files(folder: &Path, target_size_mb: u64) {
    let target_size = target_size_mb * 1024 * 1024;
    let mut total_size = get_folder_size(folder);
    
    let mut files: Vec<PathBuf> = Vec::new();
    if let Ok(entries) = fs::read_dir(folder) {
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.is_file() {
                    files.push(path);
                }
            }
        }
    }

    files.sort_by_key(|file| {
        fs::metadata(file)
            .and_then(|metadata| metadata.created())
            .unwrap_or(UNIX_EPOCH)
    });

    for file_path in files {
        if total_size < target_size {
            break;
        }
        if let Ok(metadata) = fs::metadata(&file_path) {
            if fs::remove_file(&file_path).is_ok() {
                total_size = total_size.saturating_sub(metadata.len());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile;

    #[test]
    fn test_get_folder_size_empty() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let size = get_folder_size(tmp_dir.path());
        assert_eq!(size, 0);
    }

    #[test]
    fn test_get_folder_size_with_files() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let file_path = tmp_dir.path().join("test.txt");
        let mut f = File::create(&file_path).unwrap();
        f.write_all(b"123456").unwrap(); // 6 字节
        assert_eq!(get_folder_size(tmp_dir.path()), 6);
    }

    #[test]
    fn test_delete_oldest_files() {
        let tmp_dir = tempfile::tempdir().unwrap();
        // 创建两个文件
        let file1 = tmp_dir.path().join("old.txt");
        let file2 = tmp_dir.path().join("new.txt");

        {
            let mut f = File::create(&file1).unwrap();
            f.write_all(b"1234567890").unwrap(); // 10字节
        }
        {
            let mut f = File::create(&file2).unwrap();
            f.write_all(b"abcdef").unwrap(); // 6字节
        }
        
        delete_oldest_files(tmp_dir.path(), 0); // 0MB = 0字节, 逼它删完
        assert_eq!(get_folder_size(tmp_dir.path()), 0);
    }
}