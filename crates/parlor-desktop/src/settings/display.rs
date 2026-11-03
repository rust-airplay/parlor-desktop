// SPDX-License-Identifier: GPL-3.0-or-later
//! The picture a client is asked to send.

use serde::{Deserialize, Serialize};

use super::value::Problem;

/// The display a client reads from the receiver, and therefore what it sends.
///
/// The protocol has no field that resizes a stream afterwards: a client sends a mirror that
/// matches the display it was offered. Changing these values is how a receiver asks for a
/// smaller picture, and it takes effect for the next session.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct DisplaySettings {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Refresh rate in hertz.
    pub refresh_rate: f64,
    /// The highest frame rate a client should send.
    pub max_fps: u32,
    /// Whether the picture is overscanned.
    pub overscanned: bool,
}

impl Default for DisplaySettings {
    fn default() -> Self {
        Self { width: 1920, height: 1080, refresh_rate: 60.0, max_fps: 30, overscanned: false }
    }
}

impl DisplaySettings {
    /// The smallest picture a client can be asked for.
    pub const MIN_SIDE: u32 = 240;
    /// The largest picture a client can be asked for.
    pub const MAX_SIDE: u32 = 7680;
    /// The frame rates the interface offers, slow enough to be cheap and fast enough to
    /// be worth watching.
    pub const FRAME_RATES: [u32; 6] = [24, 25, 30, 50, 60, 120];

    /// The display the receiver advertises.
    pub fn display(&self) -> rust_airplay::server::Display {
        rust_airplay::server::Display {
            width: u64::from(self.width),
            height: u64::from(self.height),
            refresh_rate: self.refresh_rate,
            max_fps: u64::from(self.max_fps),
            overscanned: self.overscanned,
        }
    }

    /// The problems with these values, if any.
    pub fn problems(&self) -> Vec<Problem> {
        let mut problems = Vec::new();
        for (field, side) in [("width", self.width), ("height", self.height)] {
            if !(Self::MIN_SIDE..=Self::MAX_SIDE).contains(&side) {
                problems.push(Problem::new(
                    "display",
                    field,
                    format!("must be between {} and {} pixels", Self::MIN_SIDE, Self::MAX_SIDE),
                ));
            }
        }
        if !(1..=240).contains(&self.max_fps) {
            problems.push(Problem::new("display", "max_fps", "must be between 1 and 240"));
        }
        if !(1.0..=240.0).contains(&self.refresh_rate) {
            problems.push(Problem::new(
                "display",
                "refresh_rate",
                "must be between 1 and 240 hertz",
            ));
        }
        problems
    }
}

/// The picture sizes the home page offers, largest first.
///
/// A free size is always available; these are the ones a person recognizes.
pub const PRESETS: [(u32, u32); 8] = [
    (3840, 2160),
    (2560, 1440),
    (1920, 1200),
    (1920, 1080),
    (1680, 1050),
    (1440, 900),
    (1280, 720),
    (1024, 768),
];
