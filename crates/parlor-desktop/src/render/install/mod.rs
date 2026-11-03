// SPDX-License-Identifier: GPL-3.0-or-later
//! Installing a backend, when the machine has none.
//!
//! The two backends are installed differently because they are different things. FFmpeg is
//! two programs in an archive; GStreamer is a runtime with plugins in an installer that can
//! be unpacked without administrator rights. Both land in the application's own data
//! directory, so a person who removes the application has nothing left behind.
//!
//! Everything here blocks: a download is a network read, an unpack is a disk write. The
//! caller runs it on a thread and reads [Progress] as it goes.

mod download;
mod extract;
mod ffmpeg;
mod gstreamer;

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use crate::settings::store::Paths;

/// A backend that can be installed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    /// The FFmpeg command line programs.
    Ffmpeg,
    /// The GStreamer runtime.
    GStreamer,
}

impl Target {
    /// The directory name this backend is installed under.
    pub fn directory(self) -> &'static str {
        match self {
            Self::Ffmpeg => "ffmpeg",
            Self::GStreamer => "gstreamer",
        }
    }
}

/// What an installation is doing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Progress {
    /// Choosing a location and preparing it.
    Preparing,
    /// Reading the archive from the network.
    Downloading {
        /// Bytes received so far.
        received: u64,
        /// The size the server promised, when it promised one.
        total: Option<u64>,
    },
    /// Unpacking what was downloaded.
    Extracting,
    /// Running the program once, to see that it works.
    Verifying,
    /// Installed and working.
    Done {
        /// The version the program reports.
        version: String,
        /// What to tell the interface about the placement.
        location: PathBuf,
    },
    /// The installation failed, and nothing usable was left behind.
    Failed(String),
}

impl Progress {
    /// Whether the installation has finished, well or badly.
    pub fn is_final(&self) -> bool {
        matches!(self, Self::Done { .. } | Self::Failed(_))
    }

    /// How far along it is, as a number from 0 to 1.
    pub fn fraction(&self) -> f64 {
        match self {
            Self::Preparing => 0.02,
            Self::Downloading { received, total } => {
                let total = total.unwrap_or(0);
                if total == 0 { 0.05 } else { 0.05 + 0.85 * (*received as f64 / total as f64) }
            }
            Self::Extracting => 0.92,
            Self::Verifying => 0.97,
            Self::Done { .. } => 1.0,
            Self::Failed(_) => 0.0,
        }
    }
}

/// One installation, with the switch that stops it.
pub struct Installer {
    /// Which backend is being installed.
    pub target: Target,
    /// Where the application keeps its files.
    pub paths: Paths,
    /// Set to stop: the download loop reads it, and the unpack is one step.
    pub cancel: Arc<AtomicBool>,
}

impl Installer {
    /// An installation of one backend.
    pub fn new(target: Target, paths: Paths) -> Self {
        Self { target, paths, cancel: Arc::new(AtomicBool::new(false)) }
    }

    /// The switch a cancel button sets.
    pub fn cancel_handle(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.cancel)
    }

    /// Installs the backend, reporting as it goes.
    ///
    /// A failure removes what this call created, so a second attempt starts from the same
    /// state as the first. A cancel is a failure with its own message.
    pub fn install(&self, report: &mut dyn FnMut(Progress)) {
        report(Progress::Preparing);
        let outcome = match self.target {
            Target::Ffmpeg => ffmpeg::install(&self.paths, &self.cancel, report),
            Target::GStreamer => gstreamer::install(&self.paths, &self.cancel, report),
        };
        match outcome {
            Ok((version, location)) => report(Progress::Done { version, location }),
            Err(message) => {
                let _ = remove(self.target, &self.paths);
                report(Progress::Failed(message));
            }
        }
    }
}

/// Whether an installation of this backend is present.
pub fn installed(target: Target, paths: &Paths) -> bool {
    match target {
        Target::Ffmpeg => {
            let bin = paths.backend(target.directory()).join("bin");
            bin.join("ffmpeg.exe").is_file()
        }
        Target::GStreamer => {
            paths.backend(target.directory()).join("bin").join("gst-launch-1.0.exe").is_file()
        }
    }
}

/// Removes an installed backend.
///
/// # Errors
///
/// Returns the operating system's message when the directory cannot be removed, which is
/// what a file still open in a running session looks like.
pub fn remove(target: Target, paths: &Paths) -> Result<(), String> {
    let directory = paths.backend(target.directory());
    match std::fs::remove_dir_all(&directory) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("cannot remove {}: {error}", directory.display())),
    }
}
