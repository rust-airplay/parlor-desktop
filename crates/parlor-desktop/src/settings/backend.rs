// SPDX-License-Identifier: GPL-3.0-or-later
//! What plays the media, and the choices each backend offers.
//!
//! Two backends are supported and they are not interchangeable: one decodes with the
//! FFmpeg command line tools, the other links GStreamer and runs pipelines. What a user
//! can configure differs with the choice, which is why each backend has its own settings.

use serde::{Deserialize, Serialize};

use super::value::Problem;

/// The playback backend a session uses.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum BackendKind {
    /// GStreamer, loaded from a plugin at run time.
    #[default]
    #[serde(rename = "gstreamer")]
    GStreamer,
    /// The FFmpeg command line tools.
    Ffmpeg,
}

/// The coding a warm video sink is started for.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum VideoCodecChoice {
    /// H.264.
    #[default]
    H264,
    /// H.265, also called HEVC.
    H265,
}

impl VideoCodecChoice {
    /// The coding the library names this way.
    pub fn codec(self) -> rust_airplay::VideoCodec {
        match self {
            Self::H264 => rust_airplay::VideoCodec::H264,
            Self::H265 => rust_airplay::VideoCodec::H265,
        }
    }
}

/// The FFmpeg backend's choices.
///
/// An empty program name means "find it on the machine", which is tried in the directory
/// the application installed into first and on the PATH second.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default)]
pub struct FfmpegSettings {
    /// The ffmpeg executable that decodes audio.
    pub decoder: String,
    /// The command that plays audio, such as "ffplay -nodisp".
    pub audio_sink: String,
    /// The command that displays video, such as "ffplay -framedrop".
    pub video_sink: String,
    /// Whether the video program is kept between sessions.
    pub warm_video: bool,
    /// The coding a warm video program is started for.
    pub warm_codec: VideoCodecChoice,
    /// Whether the backend honours a spacing asked for through the video stream.
    pub pacing: bool,
    /// Whether the video player is asked to decode on the GPU.
    pub hardware: bool,
}

impl Default for FfmpegSettings {
    fn default() -> Self {
        Self {
            decoder: String::new(),
            audio_sink: String::new(),
            video_sink: String::new(),
            warm_video: false,
            warm_codec: VideoCodecChoice::H264,
            pacing: true,
            // A new installation decodes on the GPU when the build has an accelerator; the
            // renderer falls back to software when it does not.
            hardware: true,
        }
    }
}

/// The GStreamer backend's choices.
///
/// Empty values mean "whatever the backend would choose", which is how a machine whose
/// sinks are unusual still works without being configured first.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default)]
pub struct GStreamerSettings {
    /// The gst-launch-1.0 executable used for video, or empty to find it.
    pub launch_path: String,
    /// The audio sink, as a pipeline fragment such as "wasapisink".
    pub audio_sink: String,
    /// The video sink, as a pipeline fragment such as "d3d11videosink".
    pub video_sink: String,
    /// The element that parses H.264.
    pub h264_parser: String,
    /// The element that decodes H.264.
    pub h264_decoder: String,
    /// The element that parses H.265.
    pub h265_parser: String,
    /// The element that decodes H.265.
    pub h265_decoder: String,
    /// Whether the video pipeline is kept between sessions.
    pub warm_video: bool,
    /// Whether the backend honours a spacing asked for through the video stream.
    pub pacing: bool,
}

impl Default for GStreamerSettings {
    fn default() -> Self {
        Self {
            launch_path: String::new(),
            audio_sink: String::new(),
            video_sink: String::new(),
            h264_parser: String::new(),
            h264_decoder: String::new(),
            h265_parser: String::new(),
            h265_decoder: String::new(),
            warm_video: false,
            pacing: true,
        }
    }
}

/// Which backend a session uses, and how it is configured.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default)]
pub struct BackendSettings {
    /// The backend a session asks for.
    pub kind: BackendKind,
    /// The FFmpeg backend's choices.
    pub ffmpeg: FfmpegSettings,
    /// The GStreamer backend's choices.
    pub gstreamer: GStreamerSettings,
}

impl BackendSettings {
    /// The problems with these values, if any.
    pub fn problems(&self) -> Vec<Problem> {
        let mut problems = Vec::new();
        if self.kind == BackendKind::Ffmpeg {
            for (field, command) in [
                ("audio_sink", self.ffmpeg.audio_sink.as_str()),
                ("video_sink", self.ffmpeg.video_sink.as_str()),
            ] {
                if !command.trim().is_empty() && command.split_whitespace().next().is_none() {
                    problems.push(Problem::new("backend", field, "must name a program"));
                }
            }
        }
        if self.kind == BackendKind::GStreamer {
            for (field, fragment) in [
                ("audio_sink", self.gstreamer.audio_sink.as_str()),
                ("video_sink", self.gstreamer.video_sink.as_str()),
            ] {
                if fragment.contains('!') {
                    problems.push(Problem::new(
                        "backend",
                        field,
                        "must be one element, not a pipeline",
                    ));
                }
            }
        }
        problems
    }
}
