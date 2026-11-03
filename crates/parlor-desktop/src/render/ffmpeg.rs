// SPDX-License-Identifier: GPL-3.0-or-later
//! The FFmpeg backend, built from the programs a machine has.
//!
//! This backend links nothing: it runs ffmpeg to decode and a player to present. That is
//! what makes it installable after the fact - the programs a missing runtime needs are two
//! files, not a rebuild - and it is also why a session can start with this backend on a
//! machine where GStreamer was never installed.

use std::sync::Arc;

use rust_airplay::Renderer;
use rust_airplay_ffmpeg::{FfmpegRenderer, Options, Program};

use crate::render::Error;
use crate::render::detect;
use crate::settings::Settings;
use crate::settings::store::Paths;

/// Builds a renderer around the FFmpeg programs this machine has.
///
/// # Errors
///
/// Returns [Error::Ffmpeg] when ffmpeg or ffplay is absent, or when the backend refuses
/// the command lines the settings asked for.
pub fn build(settings: &Settings, paths: &Paths) -> Result<Arc<dyn Renderer>, Error> {
    let programs = detect::ffmpeg_programs(settings, paths).ok_or_else(|| {
        Error::Ffmpeg("ffmpeg and ffplay were not found; install the FFmpeg backend".to_owned())
    })?;
    let options = Options::new(program(&programs.audio_sink), program(&programs.video_sink))
        .with_decoder(programs.decoder.to_string_lossy().into_owned())
        .with_warm_video(settings.backend.ffmpeg.warm_video)
        .with_warm_codec(settings.backend.ffmpeg.warm_codec.codec())
        .with_pacing(settings.backend.ffmpeg.pacing);
    let renderer =
        FfmpegRenderer::with_options(options).map_err(|error| Error::Ffmpeg(error.to_string()))?;
    Ok(Arc::new(renderer))
}

/// A command line as the backend's program type.
fn program(words: &[String]) -> Program {
    let (first, rest) = words.split_first().expect("a sink command has a program");
    Program::new(first.clone()).args(rest.iter().cloned())
}

#[cfg(test)]
mod tests {
    use super::program;

    #[test]
    fn a_command_keeps_its_arguments() {
        let words = vec!["C:/ffmpeg/bin/ffplay.exe".to_owned(), "-nodisp".to_owned()];
        let program = program(&words);
        assert_eq!(program.name(), "C:/ffmpeg/bin/ffplay.exe");
        assert_eq!(program.arguments(), ["-nodisp"]);
    }
}
