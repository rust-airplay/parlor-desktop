// SPDX-License-Identifier: GPL-3.0-or-later
//! Installing the FFmpeg programs.
//!
//! The archive is the distributable build the project publishes: one zip with the
//! programs in a bin directory. Only the two programs a session runs are kept; the
//! documentation, the headers and the third program in the archive are not this
//! program's business.

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;

use crate::render::detect;
use crate::render::install::Progress;
use crate::render::install::download;
use crate::render::install::extract;
use crate::settings::store::Paths;

/// The archive installed, which is the release build with the usual codecs.
pub const URL: &str = "https://www.gyan.dev/ffmpeg/builds/ffmpeg-release-essentials.zip";

/// The programs a session runs, which are the only two the archive needs to provide.
const WANTED: [&str; 2] = ["ffmpeg.exe", "ffplay.exe"];

/// Installs FFmpeg under the application's directory.
///
/// # Errors
///
/// Returns a sentence when the download, the unpack or the verification fails.
pub fn install(
    paths: &Paths,
    cancelled: &AtomicBool,
    report: &mut dyn FnMut(Progress),
) -> Result<(String, PathBuf), String> {
    let target = paths.backend("ffmpeg");
    let _ = std::fs::remove_dir_all(&target);
    let archive = paths.backend("ffmpeg-download").join("ffmpeg.zip");
    let download_result = download::to_file(URL, &archive, cancelled, |received, total| {
        report(Progress::Downloading { received, total });
    });
    if let Err(message) = download_result {
        let _ = std::fs::remove_file(&archive);
        return Err(message);
    }
    report(Progress::Extracting);
    let binary = target.join("bin");
    let unpacked = extract::zip_files(&archive, &WANTED, &binary);
    let _ = std::fs::remove_file(&archive);
    let _ = std::fs::remove_dir_all(paths.backend("ffmpeg-download"));
    unpacked?;
    report(Progress::Verifying);
    let decoder = binary.join("ffmpeg.exe");
    let version =
        detect::version_of(&decoder).ok_or_else(|| format!("{} did not run", decoder.display()))?;
    Ok((version, binary))
}

#[cfg(test)]
mod tests {
    use super::install;
    use crate::settings::store::Paths;
    use std::sync::atomic::AtomicBool;

    #[test]
    #[ignore = "downloads the release archive"]
    fn an_installation_produces_a_working_decoder() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let paths = Paths::at(directory.path());
        let cancelled = AtomicBool::new(false);
        let mut report = |_progress| {};
        let (version, location) =
            install(&paths, &cancelled, &mut report).expect("an installation");
        assert!(location.join("ffmpeg.exe").is_file());
        assert!(location.join("ffplay.exe").is_file());
        assert!(version.to_lowercase().contains("ffmpeg"), "{version}");
    }
}
