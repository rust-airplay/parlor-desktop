// SPDX-License-Identifier: GPL-3.0-or-later
//! Unpacking what was downloaded.

use std::fs::File;
use std::path::{Path, PathBuf};

/// Copies the named files out of a zip archive into a directory.
///
/// Only the named files are written. An archive from a download is not a directory to
/// unpack into the application's data directory: it carries names this program did not
/// choose, and a traversal in one of them is the oldest bug in unpacking.
///
/// # Errors
///
/// Returns a sentence when the archive cannot be read, a wanted file is absent, or a file
/// cannot be written.
pub fn zip_files(
    archive: &Path,
    wanted: &[&str],
    destination: &Path,
) -> Result<Vec<PathBuf>, String> {
    let file = File::open(archive)
        .map_err(|error| format!("cannot read {}: {error}", archive.display()))?;
    let mut zip = zip::ZipArchive::new(file)
        .map_err(|error| format!("{} is not a zip archive: {error}", archive.display()))?;
    std::fs::create_dir_all(destination)
        .map_err(|error| format!("cannot create {}: {error}", destination.display()))?;
    let mut written = Vec::new();
    for index in 0..zip.len() {
        let mut entry = zip
            .by_index(index)
            .map_err(|error| format!("cannot read an entry of {}: {error}", archive.display()))?;
        if entry.is_dir() {
            continue;
        }
        let Some(name) = entry
            .enclosed_name()
            .and_then(|path| path.file_name().map(|name| name.to_string_lossy().into_owned()))
        else {
            continue;
        };
        if !wanted.iter().any(|wanted| name.eq_ignore_ascii_case(wanted)) {
            continue;
        }
        let target = destination.join(&name);
        let mut output = File::create(&target)
            .map_err(|error| format!("cannot write {}: {error}", target.display()))?;
        std::io::copy(&mut entry, &mut output)
            .map_err(|error| format!("cannot write {}: {error}", target.display()))?;
        written.push(target);
    }
    for name in wanted {
        if !written
            .iter()
            .any(|path| path.file_name().is_some_and(|written| written.eq_ignore_ascii_case(name)))
        {
            return Err(format!("{} has no {name}", archive.display()));
        }
    }
    Ok(written)
}

/// Unpacks an installer package into a directory without installing it.
///
/// An administrative install is what Windows Installer calls extracting: it writes the
/// package's files in the layout the installer would have used, and changes nothing about
/// the machine. That is the difference between this and running the vendor's installer,
/// which needs administrator rights and leaves an uninstall entry behind.
///
/// # Errors
///
/// Returns a sentence when the installer cannot be run, or when what it unpacked has no
/// payload the caller can use.
pub fn msi_payload(archive: &Path, staging: &Path) -> Result<PathBuf, String> {
    if staging.exists() {
        std::fs::remove_dir_all(staging)
            .map_err(|error| format!("cannot clear {}: {error}", staging.display()))?;
    }
    std::fs::create_dir_all(staging)
        .map_err(|error| format!("cannot create {}: {error}", staging.display()))?;
    let log = staging.with_extension("log");
    let status = std::process::Command::new("msiexec")
        .arg("/a")
        .arg(archive)
        .arg("/qn")
        .arg(format!("TARGETDIR={}", staging.display()))
        .arg("/L*v")
        .arg(&log)
        .status()
        .map_err(|error| format!("cannot run msiexec: {error}"))?;
    if !status.success() {
        return Err(format!("the installer exited with {status}; its log is {}", log.display()));
    }
    // The package decides its own layout, so the payload is found rather than assumed: the
    // first directory that holds a GStreamer runtime is the one to keep.
    find_payload(staging, "gst-launch-1.0.exe").ok_or_else(|| {
        format!("the extracted package has no GStreamer runtime in {}", staging.display())
    })
}

/// The first directory under [root] whose bin holds [program].
fn find_payload(root: &Path, program: &str) -> Option<PathBuf> {
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        if directory.join("bin").join(program).is_file() {
            return Some(directory);
        }
        let entries = std::fs::read_dir(&directory).ok()?;
        for entry in entries.flatten() {
            if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
                pending.push(entry.path());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::find_payload;

    #[test]
    fn a_payload_is_found_by_its_program() {
        let root = tempfile::tempdir().expect("a temporary directory");
        let runtime = root.path().join("nested").join("runtime");
        std::fs::create_dir_all(runtime.join("bin")).expect("a bin directory");
        std::fs::write(runtime.join("bin").join("gst-launch-1.0.exe"), b"").expect("a program");
        assert_eq!(find_payload(root.path(), "gst-launch-1.0.exe"), Some(runtime));
        assert_eq!(find_payload(root.path(), "nothing.exe"), None);
    }
}
