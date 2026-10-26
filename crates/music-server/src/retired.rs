//! What earlier versions of the studio downloaded into its data folder and no part of it reads any more: removed at
//! start, so they do not keep gigabytes that no row of the setup screen shows (Speech Studio's `RETIRED`).

use std::fs;
use std::path::Path;

/// Paths under the studio's data folder, files or folders.
pub const RETIRED: &[&str] = &[
    // the ONNX Parakeet of 3.6 and earlier; karaoke recognises on transcribe.cpp
    "karaoke/models/parakeet",
    "karaoke/models/parakeet-fp32",
    "karaoke/models/parakeet-ultra",
];

/// Removes the retired paths under `data_root`. A link (another program's folder linked in) loses only the link; a
/// hard-linked file stays with the program that shares it.
pub fn remove_retired(data_root: &Path) {
    for relative in RETIRED {
        let path = data_root.join(relative);
        let Ok(meta) = fs::symlink_metadata(&path) else { continue };
        let removed = if meta.file_type().is_symlink() {
            fs::remove_dir(&path).or_else(|_| fs::remove_file(&path))
        } else if meta.is_dir() {
            fs::remove_dir_all(&path)
        } else {
            fs::remove_file(&path)
        };
        match removed {
            Ok(()) => eprintln!("removed the retired {}", path.display()),
            Err(error) => eprintln!("[ERROR] could not remove the retired {}: {error}", path.display()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retired_paths_go_and_current_ones_stay() {
        let root = tempfile::tempdir().unwrap();
        for relative in RETIRED {
            let path = root.path().join(relative);
            fs::create_dir_all(&path).unwrap();
            fs::write(path.join("encoder-model.int8.onnx"), b"x").unwrap();
        }
        let kept = root.path().join("karaoke/models/asr-gguf-parakeet-v3/parakeet-tdt-0.6b-v3-Q8_0.gguf");
        fs::create_dir_all(kept.parent().unwrap()).unwrap();
        fs::write(&kept, b"x").unwrap();
        remove_retired(root.path());
        assert!(RETIRED.iter().all(|relative| !root.path().join(relative).exists()));
        assert!(kept.is_file());
        remove_retired(root.path());
    }
}
