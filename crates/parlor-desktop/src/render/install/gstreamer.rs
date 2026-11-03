// SPDX-License-Identifier: GPL-3.0-or-later
//! Installing the GStreamer runtime.
//!
//! The runtime is an installer package. It is unpacked rather than installed: an
//! administrative install writes the same files into a directory the caller names and
//! changes nothing about the machine, which is what makes this a per-user install with no
//! administrator prompt.
//!
//! The version is pinned. A newer runtime is a different set of plugins than the plugin
//! was built against, and this is not the place to discover that.

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::AtomicBool;

use crate::render::install::Progress;
use crate::render::install::download;
use crate::render::install::extract;
use crate::settings::store::Paths;

/// The installer package for the 64-bit MSVC runtime.
pub const URL: &str = "https://gstreamer.freedesktop.org/data/pkg/windows/1.26.11/msvc/gstreamer-1.0-msvc-x86_64-1.26.11.msi";

/// Installs GStreamer under the application's directory.
///
/// # Errors
///
/// Returns a sentence when the download, the unpack or the verification fails.
pub fn install(
    paths: &Paths,
    cancelled: &AtomicBool,
    report: &mut dyn FnMut(Progress),
) -> Result<(String, PathBuf), String> {
    let target = paths.backend("gstreamer");
    let staging = paths.backend("gstreamer-staging");
    let _ = std::fs::remove_dir_all(&target);
    let _ = std::fs::remove_dir_all(&staging);
    std::fs::create_dir_all(&staging)
        .map_err(|error| format!("cannot create {}: {error}", staging.display()))?;
    let archive = staging.join("runtime.msi");
    let download_result = download::to_file(URL, &archive, cancelled, |received, total| {
        report(Progress::Downloading { received, total });
    });
    if let Err(message) = download_result {
        let _ = std::fs::remove_dir_all(&staging);
        return Err(message);
    }
    report(Progress::Extracting);
    let unpacked = extract::msi_payload(&archive, &staging.join("payload"));
    if let Err(message) = unpacked {
        let _ = std::fs::remove_dir_all(&staging);
        return Err(message);
    }
    let unpacked = unpacked.unwrap_or_default();
    move_contents(&unpacked, &target)?;
    let _ = std::fs::remove_dir_all(&staging);
    report(Progress::Verifying);
    let launch = target.join("bin").join("gst-launch-1.0.exe");
    let version = version_of(&launch, &target.join("bin"))
        .ok_or_else(|| format!("{} did not run", launch.display()))?;
    Ok((version, target))
}

/// Moves every entry of one directory into another.
///
/// A rename is what is wanted, and a move inside one filesystem must not copy a runtime of
/// several hundred megabytes; the content is under the same data directory, so a rename
/// stays on one volume.
fn move_contents(source: &std::path::Path, destination: &std::path::Path) -> Result<(), String> {
    std::fs::create_dir_all(destination)
        .map_err(|error| format!("cannot create {}: {error}", destination.display()))?;
    let entries = std::fs::read_dir(source)
        .map_err(|error| format!("cannot read {}: {error}", source.display()))?;
    for entry in entries.flatten() {
        let destination = destination.join(entry.file_name());
        std::fs::rename(entry.path(), &destination)
            .map_err(|error| format!("cannot move into {}: {error}", destination.display()))?;
    }
    Ok(())
}

/// Runs the launcher once, with the runtime beside it, and reads its version.
fn version_of(launch: &std::path::Path, binary: &std::path::Path) -> Option<String> {
    let path = std::env::var_os("PATH").unwrap_or_default();
    let mut directories = vec![binary.to_path_buf()];
    directories.extend(std::env::split_paths(&path));
    let joined = std::env::join_paths(directories).ok()?;
    let output = Command::new(launch)
        .arg("--version")
        .env("PATH", joined)
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .next()
        .map(|line| line.trim().to_owned())
        .filter(|line| !line.is_empty())
}
