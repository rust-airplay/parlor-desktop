// SPDX-License-Identifier: GPL-3.0-or-later
//! What is recorded, for a machine that is behaving oddly.

use serde::{Deserialize, Serialize};

/// How much of the receiver's log is kept.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum LogLevel {
    /// Nothing.
    Off,
    /// Failures only.
    Error,
    /// Failures and warnings.
    Warn,
    /// The session's milestones, which is the default.
    #[default]
    Info,
    /// Every request and every pipeline decision.
    Debug,
    /// Everything, including the protocol's own detail.
    Trace,
}

impl LogLevel {
    /// The filter this level installs.
    pub fn filter(self) -> log::LevelFilter {
        match self {
            Self::Off => log::LevelFilter::Off,
            Self::Error => log::LevelFilter::Error,
            Self::Warn => log::LevelFilter::Warn,
            Self::Info => log::LevelFilter::Info,
            Self::Debug => log::LevelFilter::Debug,
            Self::Trace => log::LevelFilter::Trace,
        }
    }
}

/// The diagnosis files a session writes.
///
/// Each one is off by default. A dump is a file a person asked for, and a receiver that
/// wrote one unasked would be spending a user's disk on a guess.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default)]
pub struct DiagnosticsSettings {
    /// How much of the log is kept.
    pub log_level: LogLevel,
    /// Whether the request trace is appended to a file.
    pub trace_log: bool,
    /// Whether the first video frames are written to a file.
    pub dump_video: bool,
    /// Whether the audio payloads are written to a file.
    pub dump_audio: bool,
    /// Whether a client's own streaming reports are written to the log.
    pub streaming_reports: bool,
    /// Whether a SETUP body is reported, which includes the media key.
    pub setup_body: bool,
}

impl Default for DiagnosticsSettings {
    fn default() -> Self {
        Self {
            log_level: LogLevel::Info,
            trace_log: false,
            dump_video: false,
            dump_audio: false,
            streaming_reports: false,
            setup_body: false,
        }
    }
}
