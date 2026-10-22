//! What a download put on disk, and which pin it came from.
//!
//! Beside every archive or file a download fetched lies `<that path>.json`, the
//! record Dub Studio keeps too: the archive's SHA-256, the pinned URL it came
//! from and every file it put down with its size. An asset counts as installed
//! when its record names the URL of today's pin and every recorded file is
//! there at its recorded size, so a runtime of an earlier pin under the same
//! file names is found out instead of being taken for the new one.
//!
//! What was downloaded before records existed has none. A model counts by its
//! presence, as before; a runtime is taken over only when the one file the pin
//! names (`KEY_FILES`) has exactly the pinned size, and is otherwise fetched
//! again.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct InstallRecord {
    pub sha256: String,
    pub url: String,
    pub files: Vec<RecordedFile>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RecordedFile {
    /// Relative to the root the record was written for, with `/`.
    pub path: String,
    pub size: u64,
}

/// For a runtime downloaded before records existed: the file of the pinned
/// archive that tells its version apart, relative to where the archive
/// unpacks or picks into, and that file's size in the archive of the pin.
const KEY_FILES: &[(&str, &str, u64)] = &[
    ("https://github.com/ggml-org/llama.cpp/releases/download/b11236/llama-b11236-bin-win-cuda-13.4-x64.zip", "llama-server-impl.dll", 8_917_504),
    ("https://github.com/ggml-org/llama.cpp/releases/download/b11236/cudart-llama-bin-win-cuda-13.4-x64.zip", "cudart64_13.dll", 551_024),
    ("https://github.com/ggml-org/llama.cpp/releases/download/b11236/llama-b11236-bin-win-cuda-12.4-x64.zip", "llama-server-impl.dll", 8_917_504),
    ("https://github.com/ggml-org/llama.cpp/releases/download/b11236/cudart-llama-bin-win-cuda-12.4-x64.zip", "cudart64_12.dll", 553_984),
    ("https://github.com/ggml-org/llama.cpp/releases/download/b11236/llama-b11236-bin-win-vulkan-x64.zip", "llama-server-impl.dll", 8_917_504),
    ("https://github.com/ggml-org/llama.cpp/releases/download/b11236/llama-b11236-bin-win-cpu-x64.zip", "llama-server-impl.dll", 8_917_504),
    ("https://developer.download.nvidia.com/compute/cuda/redist/libcublas/windows-x86_64/libcublas-windows-x86_64-13.5.1.27-archive.zip", "cublasLt64_13.dll", 460_301_424),
    ("https://developer.download.nvidia.com/compute/cuda/redist/libcublas/windows-x86_64/libcublas-windows-x86_64-12.9.1.4-archive.zip", "cublasLt64_12.dll", 668_669_952),
    ("https://github.com/Purfview/whisper-standalone-win/releases/download/faster-whisper/Whisper-Faster_r192.3_windows.zip", "whisper-faster.exe", 88_436_526),
    ("https://developer.download.nvidia.com/compute/cuda/redist/libcublas/windows-x86_64/libcublas-windows-x86_64-11.11.3.6-archive.zip", "cublas64_11.dll", 88_668_160),
    ("https://developer.download.nvidia.com/compute/cudnn/redist/cudnn/windows-x86_64/cudnn-windows-x86_64-8.9.7.29_cuda11-archive.zip", "cudnn64_8.dll", 288_296),
    ("https://github.com/microsoft/onnxruntime/releases/download/v1.30.0/onnxruntime-win-x64-gpu_cuda12-1.30.0.zip", "onnxruntime_providers_cuda.dll", 352_453_432),
    ("https://developer.download.nvidia.com/compute/cuda/redist/libcublas/windows-x86_64/libcublas-windows-x86_64-12.9.2.10-archive.zip", "cublasLt64_12.dll", 668_673_536),
    ("https://developer.download.nvidia.com/compute/cuda/redist/cuda_cudart/windows-x86_64/cuda_cudart-windows-x86_64-12.9.79-archive.zip", "cudart64_12.dll", 583_680),
    ("https://developer.download.nvidia.com/compute/cuda/redist/libcufft/windows-x86_64/libcufft-windows-x86_64-11.4.1.4-archive.zip", "cufft64_11.dll", 287_136_768),
    ("https://developer.download.nvidia.com/compute/cudnn/redist/cudnn/windows-x86_64/cudnn-windows-x86_64-9.25.0.15_cuda12-archive.zip", "cudnn64_9.dll", 267_888),
    ("https://github.com/microsoft/onnxruntime/releases/download/v1.30.0/onnxruntime-win-x64-1.30.0.zip", "onnxruntime.dll", 16_462_648),
    ("https://api.nuget.org/v3-flatcontainer/microsoft.ml.onnxruntime.directml/1.24.4/microsoft.ml.onnxruntime.directml.1.24.4.nupkg", "onnxruntime.dll", 17_328_152),
    ("https://api.nuget.org/v3-flatcontainer/microsoft.ai.directml/1.15.4/microsoft.ai.directml.1.15.4.nupkg", "DirectML.dll", 18_527_776),
    ("https://github.com/chenmozhijin/BSRoformer.cpp/releases/download/v0.1.0/BSRoformer-windows-cuda-13.1.0.zip", "bs_roformer-cli.exe", 182_784),
    ("https://github.com/chenmozhijin/BSRoformer.cpp/releases/download/v0.1.0/BSRoformer-windows-x64-msvc.zip", "bs_roformer-cli.exe", 183_808),
];

/// The record of the archive or file at `path`.
pub fn record_path(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(".json");
    PathBuf::from(name)
}

pub fn read(record: &Path) -> Option<InstallRecord> {
    let text = fs::read_to_string(record).ok()?;
    match serde_json::from_str(&text) {
        Ok(parsed) => Some(parsed),
        Err(error) => {
            eprintln!("[ERROR] the install record {} is unreadable ({error}); its download counts as not installed", record.display());
            None
        }
    }
}

/// Whether the record at `record` says `url` was installed and every file it
/// put under `root` is there at its size. None when there is no record.
pub fn matches(record: &Path, url: &str, root: &Path) -> Option<bool> {
    let parsed = read(record)?;
    Some(
        parsed.url == url
            && !parsed.files.is_empty()
            && parsed.files.iter().all(|file| fs::metadata(root.join(&file.path)).is_ok_and(|meta| meta.is_file() && meta.len() == file.size)),
    )
}

/// The key file of a pinned runtime archive and its size, when the pin names one.
pub fn key_file(url: &str) -> Option<(&'static str, u64)> {
    KEY_FILES.iter().find(|(pinned, _, _)| *pinned == url).map(|(_, file, size)| (*file, *size))
}

/// Writes the record of `url` at `record`: the archive's hash and the files it
/// put down, relative to `root`.
pub fn write(record: &Path, url: &str, sha256: &str, files: &[PathBuf], root: &Path) -> Result<()> {
    let files = files
        .iter()
        .map(|path| {
            let size = fs::metadata(path).with_context(|| format!("measure {}", path.display()))?.len();
            let relative = path.strip_prefix(root).unwrap_or(path).to_string_lossy().replace('\\', "/");
            Ok(RecordedFile { path: relative, size })
        })
        .collect::<Result<Vec<_>>>()?;
    let body = serde_json::to_vec_pretty(&InstallRecord { sha256: sha256.to_string(), url: url.to_string(), files })?;
    let partial = record.with_extension("json.tmp");
    fs::write(&partial, body).with_context(|| format!("write {}", partial.display()))?;
    fs::rename(&partial, record).with_context(|| format!("write {}", record.display()))?;
    Ok(())
}

/// Deletes the files a record lists under `root`, and the record: what an
/// earlier pin put down goes before the new one is fetched.
pub fn remove_recorded(record: &Path, root: &Path) -> Result<()> {
    if let Some(parsed) = read(record) {
        for file in &parsed.files {
            let path = root.join(&file.path);
            match fs::remove_file(&path) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error).with_context(|| format!("remove {}", path.display())),
            }
        }
    }
    match fs::remove_file(record) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).with_context(|| format!("remove {}", record.display())),
    }
}

/// The SHA-256 of a file, as lower-case hex.
pub fn sha256_of(path: &Path) -> Result<String> {
    use sha2::{Digest, Sha256};
    let mut file = fs::File::open(path).with_context(|| format!("open {}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 1 << 20];
    loop {
        let read = file.read(&mut buffer).with_context(|| format!("read {}", path.display()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher.finalize().iter().map(|byte| format!("{byte:02x}")).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("install-record-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_record_of_another_pin_or_a_changed_file_does_not_match() {
        let root = scratch("match");
        let library = root.join("runtime").join("lib.dll");
        fs::create_dir_all(library.parent().unwrap()).unwrap();
        fs::write(&library, b"twelve bytes").unwrap();
        let record = record_path(&root.join("runtime.zip"));
        assert_eq!(matches(&record, "https://x/v2.zip", &root), None);
        write(&record, "https://x/v2.zip", "ab", &[library.clone()], &root).unwrap();
        assert_eq!(matches(&record, "https://x/v2.zip", &root), Some(true));
        assert_eq!(matches(&record, "https://x/v3.zip", &root), Some(false));
        fs::write(&library, b"other").unwrap();
        assert_eq!(matches(&record, "https://x/v2.zip", &root), Some(false));
        remove_recorded(&record, &root).unwrap();
        assert!(!library.exists() && !record.exists());
        let _ = fs::remove_dir_all(&root);
    }
}
