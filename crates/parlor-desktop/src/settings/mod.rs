// SPDX-License-Identifier: GPL-3.0-or-later
//! The settings a person chooses, and the defaults they start from.
//!
//! A setting is a value the application owns, not one the library infers: the receiver
//! reads no environment variable and chooses no path, so everything a session needs is
//! collected here first. The module is split by the settings page that edits it, because
//! a page and its settings are read together; the file that stores them is
//! [store](crate::settings::store).

mod appearance;
mod audio;
mod auth;
mod backend;
mod diagnostics;
mod display;
mod network;
mod receiver;
mod value;
mod video;

pub mod identity;
pub mod store;

pub use appearance::{Appearance, LanguageChoice, ThemeChoice};
pub use audio::{AudioCodecChoice, AudioSettings};
pub use auth::{AuthSettings, PinChoice};
pub use backend::{BackendKind, BackendSettings, VideoCodecChoice};
pub use diagnostics::{DiagnosticsSettings, LogLevel};
pub use display::{DisplaySettings, PRESETS};
pub use network::{NetworkSettings, PortsMode};
pub use receiver::ReceiverSettings;
pub use value::Problem;
pub use video::VideoSettings;

use serde::{Deserialize, Serialize};

/// Everything a person can change, as one document.
///
/// The document is versioned: a build that adds a field must still read the file an
/// earlier build wrote, and [store](crate::settings::store) is where that promise lives.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct Settings {
    /// The format this document was written in.
    pub schema_version: u32,
    /// The window: light, dark, and the language of every label.
    pub appearance: Appearance,
    /// What clients see, and what a session starts with.
    pub receiver: ReceiverSettings,
    /// The picture a client is asked to send.
    pub display: DisplaySettings,
    /// Who may pair, and who may start a session.
    pub auth: AuthSettings,
    /// Where the receiver listens and whether it announces itself.
    pub network: NetworkSettings,
    /// Which codings the receiver advertises.
    pub video: VideoSettings,
    /// The audio a session expects, and how long it takes.
    pub audio: AudioSettings,
    /// What plays the media.
    pub backend: BackendSettings,
    /// What is recorded for a diagnosis.
    pub diagnostics: DiagnosticsSettings,
}

impl Settings {
    /// The format this build writes.
    pub const SCHEMA_VERSION: u32 = 1;

    /// A document with every default, which is also what a first run writes.
    pub fn new() -> Self {
        Self { schema_version: Self::SCHEMA_VERSION, ..Self::default() }
    }

    /// Everything wrong with this document, in the order a person meets the fields.
    ///
    /// The list is empty when the document can start a receiver. Each problem names the
    /// settings page that holds the field, so a page can show its own.
    pub fn problems(&self) -> Vec<Problem> {
        let mut problems = Vec::new();
        problems.extend(self.receiver.problems());
        problems.extend(self.display.problems());
        problems.extend(self.auth.problems());
        problems.extend(self.network.problems());
        problems.extend(self.audio.problems());
        problems.extend(self.backend.problems());
        problems
    }
}
