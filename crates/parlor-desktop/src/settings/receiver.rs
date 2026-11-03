// SPDX-License-Identifier: GPL-3.0-or-later
//! What clients see, and how one session replaces another.

use serde::{Deserialize, Serialize};

use super::value::Problem;

/// The receiver's own identity and lifetime.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default)]
pub struct ReceiverSettings {
    /// The name a client shows in its AirPlay menu.
    pub name: String,
    /// The device identifier to announce, or empty for one derived from the seed.
    ///
    /// A client remembers a receiver by this identifier, so a receiver that changes it
    /// looks like a new device.
    pub device_id: String,
    /// Whether video is held back to keep the spacing it was captured at.
    ///
    /// Off, the newest picture is shown as soon as it arrives and a burst is shown as a
    /// burst. On, a stream that arrives unevenly is spread back out, at the cost of
    /// following the device less closely.
    pub pace_video: bool,
    /// Whether a new connection from the same host replaces the previous one.
    pub nohold: bool,
    /// How long a session may go without a feedback report before it is closed, in
    /// seconds. Empty leaves a silent client connected.
    pub feedback_timeout: Option<u16>,
}

impl Default for ReceiverSettings {
    fn default() -> Self {
        Self {
            name: "Parlor".to_owned(),
            device_id: String::new(),
            pace_video: false,
            nohold: false,
            feedback_timeout: None,
        }
    }
}

impl ReceiverSettings {
    /// The longest name a client accepts.
    pub const MAX_NAME: usize = 64;

    /// The problems with these values, if any.
    pub fn problems(&self) -> Vec<Problem> {
        let mut problems = Vec::new();
        let name = self.name.trim();
        if name.is_empty() {
            problems.push(Problem::new("receiver", "name", "a client needs a name to show"));
        } else if name.chars().count() > Self::MAX_NAME {
            problems.push(Problem::new(
                "receiver",
                "name",
                format!("must be at most {} characters", Self::MAX_NAME),
            ));
        }
        if let Some(seconds) = self.feedback_timeout
            && !(1..=3600).contains(&seconds)
        {
            problems.push(Problem::new(
                "receiver",
                "feedback_timeout",
                "must be between 1 and 3600 seconds, or empty",
            ));
        }
        problems
    }

    /// The identifier to announce, when the caller named one.
    pub fn device_id(&self) -> Option<String> {
        let named = self.device_id.trim();
        if named.is_empty() { None } else { Some(named.to_owned()) }
    }

    /// The name a client sees.
    pub fn name(&self) -> &str {
        self.name.trim()
    }
}
