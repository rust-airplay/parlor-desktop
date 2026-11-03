// SPDX-License-Identifier: GPL-3.0-or-later
//! The playback backends, and how a session gets one.
//!
//! The receiver hands coded frames to a [Renderer](rust_airplay::Renderer). Which
//! implementation that is depends on a setting, and the two implementations have very
//! different requirements: the FFmpeg one runs two programs, the GStreamer one links a
//! native library and is therefore loaded from a plugin. This module is the seam that
//! hides that difference from the rest of the program.

pub mod detect;
pub mod ffmpeg;
pub mod gst;

use std::sync::Arc;

use rust_airplay::Renderer;

use crate::settings::BackendKind;
use crate::settings::Settings;

/// Why a session has no renderer.
#[derive(Debug)]
pub enum Error {
    /// The GStreamer backend could not be loaded or refused its options.
    GStreamer(String),
    /// The FFmpeg backend could not be built.
    Ffmpeg(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GStreamer(message) => write!(formatter, "GStreamer: {message}"),
            Self::Ffmpeg(message) => write!(formatter, "FFmpeg: {message}"),
        }
    }
}

impl std::error::Error for Error {}

/// Builds the renderer the settings ask for.
///
/// # Errors
///
/// Returns [Error] when the chosen backend cannot be built: its program is missing, its
/// plugin will not load, or it refuses the options. A session that has no renderer cannot
/// play, so the caller reports this instead of starting one.
pub fn build(settings: &Settings) -> Result<Arc<dyn Renderer>, Error> {
    match settings.backend.kind {
        BackendKind::GStreamer => gst::build(settings),
        BackendKind::Ffmpeg => ffmpeg::build(settings),
    }
}

#[cfg(test)]
mod tests {
    use super::build;
    use crate::settings::Settings;

    #[test]
    fn a_missing_plugin_is_reported_rather_than_panicking() {
        let mut settings = Settings::new();
        settings.backend.kind = crate::settings::BackendKind::GStreamer;
        // A machine with the plugin beside the test binary would really load it; the
        // question this test asks is only that the failure path is an error, not a crash.
        if crate::render::detect::plugin_path().is_some_and(|path| path.is_file()) {
            return;
        }
        let error = build(&settings).expect_err("a plugin that is not there");
        assert!(error.to_string().starts_with("GStreamer: "));
    }

    #[test]
    #[ignore = "needs the plugin beside the test binary and a GStreamer runtime"]
    fn the_gstreamer_plugin_builds_a_renderer() {
        let mut settings = Settings::new();
        settings.backend.kind = crate::settings::BackendKind::GStreamer;
        let renderer = build(&settings).expect("a GStreamer renderer");
        assert!(!format!("{renderer:?}").is_empty());
    }

    #[test]
    #[ignore = "needs ffmpeg and ffplay on the machine"]
    fn the_ffmpeg_backend_builds_a_renderer() {
        let mut settings = Settings::new();
        settings.backend.kind = crate::settings::BackendKind::Ffmpeg;
        let renderer = build(&settings).expect("an FFmpeg renderer");
        assert!(!format!("{renderer:?}").is_empty());
    }
}
