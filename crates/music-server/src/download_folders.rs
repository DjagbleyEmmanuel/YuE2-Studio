//! Where the files the studio downloads are kept.
//!
//! The model sets have a folder of their own (`model_manager`); everything else
//! that is fetched - the LoRAs, the stem separator, Audio to MIDI, the karaoke
//! models, the audio.cpp runtime, the assistant's own models and the trainer -
//! lives in a folder per capability under one root, the studio's data folder
//! unless the user says otherwise. Someone whose home partition is small should
//! not have to find those folders by hand, so this module owns the list, the
//! room they take, and carrying them to another disk.
//!
//! Nothing here decides when a folder changes: the service saves the choice, and
//! the folders are read at start, so a move takes effect the next time the
//! studio opens.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde::Serialize;

/// Every folder of the downloads root, with the name the window calls it by.
pub const FOLDERS: &[(&str, &str)] = &[
    ("adapters", "lora"),
    ("separation", "stems"),
    ("midi", "midi"),
    ("karaoke", "karaoke"),
    ("audiocpp", "runtime"),
    ("assistant", "assistant"),
    ("training", "training"),
    ("hotstep", "tools"),
];

/// How much one folder takes.
#[derive(Debug, Clone, Serialize)]
pub struct FolderUse {
    pub name: &'static str,
    pub bytes: u64,
}

/// A file or folder that stayed where it was, and why.
#[derive(Debug, Clone, Serialize)]
pub struct RefusedMove {
    pub file: String,
    pub error: String,
}

/// What carrying the folder to another disk did.
#[derive(Debug, Clone, Serialize)]
pub struct Relocation {
    pub previous: String,
    pub directory: String,
    pub moved_bytes: u64,
    /// The folders that came across.
    pub moved: Vec<FolderUse>,
    pub failed: Vec<RefusedMove>,
}

/// The room each folder of the root takes, in the order they are shown.
pub fn folder_sizes(root: &Path) -> Vec<FolderUse> {
    FOLDERS
        .iter()
        .map(|(name, _)| FolderUse { name, bytes: dir_bytes(&root.join(name)) })
        .collect()
}

/// Every file under `root`, added up.
pub fn dir_bytes(root: &Path) -> u64 {
    let Ok(entries) = fs::read_dir(root) else { return 0 };
    entries
        .flatten()
        .map(|entry| match entry.file_type() {
            Ok(kind) if kind.is_dir() => dir_bytes(&entry.path()),
            Ok(_) => entry.metadata().map(|meta| meta.len()).unwrap_or(0),
            Err(_) => 0,
        })
        .sum()
}

/// Carries every folder of `from` into `to`, leaving nothing behind that came
/// across.
///
/// A file already in the destination is never overwritten: what is there is
/// what the studio would use, and the copy in the old folder is left to the
/// person. A folder that will not move - a full disk, a file held open - is
/// reported and the rest still goes, so a half-finished move is a list of what
/// stayed rather than a folder that looks empty.
pub fn relocate(from: &Path, to: &Path) -> Result<Relocation> {
    if from.as_os_str().is_empty() || to.as_os_str().is_empty() {
        bail!("both folders are needed");
    }
    if from == to {
        bail!("the downloads are already in that folder");
    }
    if to.parent().is_none() || to.file_name().is_none() {
        bail!("the downloads folder must be a folder of its own, not a drive root");
    }
    fs::create_dir_all(to).with_context(|| format!("create the downloads folder {}", to.display()))?;
    let probe = to.join(".studio-write-probe");
    fs::write(&probe, b"yue2 studio").with_context(|| format!("write into {}", to.display()))?;
    let _ = fs::remove_file(&probe);

    let mut moved = Vec::new();
    let mut failed = Vec::new();
    for (name, _) in FOLDERS {
        let source = from.join(name);
        if !source.exists() {
            continue;
        }
        let bytes = dir_bytes(&source);
        if bytes == 0 {
            continue;
        }
        let target = to.join(name);
        let mut carried = 0;
        move_tree(&source, &target, name, &mut carried, &mut failed);
        if carried > 0 {
            moved.push(FolderUse { name, bytes: carried });
        }
        // an emptied folder is gone, so the old root does not look like it
        // still holds the downloads
        let _ = fs::remove_dir(&source);
    }

    Ok(Relocation {
        previous: from.display().to_string(),
        directory: to.display().to_string(),
        moved_bytes: moved.iter().map(|folder| folder.bytes).sum(),
        moved,
        failed,
    })
}

/// Takes a folder, or its contents, to another path; appends what came across.
fn move_tree(source: &Path, target: &Path, label: &str, carried: &mut u64, failed: &mut Vec<RefusedMove>) {
    if !target.exists() {
        // one rename is instant on the filesystem the files are already on
        if fs::rename(source, target).is_ok() {
            *carried += dir_bytes(target);
            return;
        }
        if let Err(error) = fs::create_dir_all(target) {
            failed.push(RefusedMove { file: label.to_string(), error: error.to_string() });
            return;
        }
    }
    let Ok(entries) = fs::read_dir(source) else {
        failed.push(RefusedMove { file: label.to_string(), error: format!("read {}", source.display()) });
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let destination = target.join(&name);
        let shown = format!("{label}/{}", name.to_string_lossy());
        let is_dir = entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false);
        if is_dir {
            move_tree(&path, &destination, &shown, carried, failed);
            continue;
        }
        if destination.exists() {
            continue;
        }
        let bytes = entry.metadata().map(|meta| meta.len()).unwrap_or(0);
        match take_across(&path, &destination) {
            Ok(()) => *carried += bytes,
            Err(error) => failed.push(RefusedMove { file: shown, error: format!("{error:#}") }),
        }
    }
}

/// Takes one file to another folder: a rename, which is instant on one
/// filesystem, and a copy otherwise, which leaves the original in place until
/// the copy is whole.
fn take_across(source: &Path, destination: &Path) -> Result<()> {
    if fs::rename(source, destination).is_ok() {
        return Ok(());
    }
    let mut staged = destination.as_os_str().to_os_string();
    staged.push(".part");
    let staged = PathBuf::from(staged);
    if let Err(error) = fs::copy(source, &staged) {
        let _ = fs::remove_file(&staged);
        return Err(error).with_context(|| format!("copy {} to {}", source.display(), destination.display()));
    }
    fs::rename(&staged, destination).with_context(|| format!("publish {}", destination.display()))?;
    let _ = fs::remove_file(source);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn workspace(label: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("downloads-folder-{label}-{}", uuid::Uuid::now_v7()));
        fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn every_folder_comes_across_with_the_files_it_holds() {
        let root = workspace("move");
        let from = root.join("studio");
        let to = root.join("elsewhere");
        // a LoRA in its own folder, the way the engine reads them
        fs::create_dir_all(from.join("adapters/yue2-blues")).unwrap();
        fs::write(from.join("adapters/yue2-blues/adapter.json"), b"{}").unwrap();
        fs::write(from.join("adapters/yue2-blues/ar.safetensors"), b"weights").unwrap();
        fs::create_dir_all(from.join("karaoke/models")).unwrap();
        fs::write(from.join("karaoke/models/whisper.bin"), b"model").unwrap();
        // a folder the studio has never used is not made on the other disk
        assert!(!from.join("midi").exists());

        let report = relocate(&from, &to).unwrap();
        assert!(report.failed.is_empty(), "{:?}", report.failed);
        assert_eq!(report.moved_bytes, 7 + 5 + 2);
        assert!(to.join("adapters/yue2-blues/ar.safetensors").is_file());
        assert_eq!(fs::read(to.join("adapters/yue2-blues/adapter.json")).unwrap(), b"{}");
        assert!(to.join("karaoke/models/whisper.bin").is_file());
        assert!(!from.join("adapters").exists(), "the old folder is emptied");
        assert!(!from.join("karaoke").exists());
        assert!(!to.join("midi").exists());
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn a_file_already_in_the_new_folder_is_kept_and_the_old_one_left_alone() {
        let root = workspace("keep");
        let from = root.join("studio");
        let to = root.join("elsewhere");
        fs::create_dir_all(from.join("adapters/mine")).unwrap();
        fs::create_dir_all(to.join("adapters/mine")).unwrap();
        fs::write(from.join("adapters/mine/ar.safetensors"), b"older").unwrap();
        fs::write(to.join("adapters/mine/ar.safetensors"), b"newer").unwrap();
        fs::write(from.join("adapters/mine/nar.safetensors"), b"other").unwrap();

        let report = relocate(&from, &to).unwrap();
        assert!(report.failed.is_empty());
        assert_eq!(fs::read(to.join("adapters/mine/ar.safetensors")).unwrap(), b"newer");
        assert!(to.join("adapters/mine/nar.safetensors").is_file());
        // what did not travel is still where the person put it
        assert!(from.join("adapters/mine/ar.safetensors").is_file());
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn the_folder_that_is_already_in_force_is_refused() {
        let root = workspace("same");
        let error = relocate(&root, &root).unwrap_err().to_string();
        assert!(error.contains("already"), "{error}");
        // and a drive root is not a downloads folder
        let error = relocate(&root, Path::new("/")).unwrap_err().to_string();
        assert!(error.contains("drive root"), "{error}");
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn what_one_folder_takes_is_added_up() {
        let root = workspace("sizes");
        fs::create_dir_all(root.join("adapters/one")).unwrap();
        fs::write(root.join("adapters/one/ar.safetensors"), b"twelve bytes").unwrap();
        fs::create_dir_all(root.join("midi/files")).unwrap();
        fs::write(root.join("midi/files/a.mid"), b"mid").unwrap();
        let sizes = folder_sizes(&root);
        let bytes = |name: &str| sizes.iter().find(|folder| folder.name == name).unwrap().bytes;
        assert_eq!(bytes("adapters"), 12);
        assert_eq!(bytes("midi"), 3);
        assert_eq!(bytes("karaoke"), 0);
        fs::remove_dir_all(root).ok();
    }
}
