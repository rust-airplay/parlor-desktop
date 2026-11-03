// SPDX-License-Identifier: GPL-3.0-or-later
//! The audio a session expects, its delay, and the level it starts at.

use serde::{Deserialize, Serialize};

use super::value::Problem;

/// The coding an audio stream is expected to carry.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum AudioCodecChoice {
    /// Apple Lossless, which is what AirPlay 1 clients send.
    Alac,
    /// MPEG-4 AAC Enhanced Low Delay, which is what a mirroring client sends.
    #[default]
    AacEld,
}

/// What a client should expect to hear, and when.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default)]
pub struct AudioSettings {
    /// The coding used when a client does not say which one it sends.
    pub codec: AudioCodecChoice,
    /// The delay reported to a client, in milliseconds.
    pub latency_ms: u32,
    /// The level a session starts at, as a percentage of the slider.
    pub initial_volume: u8,
    /// Whether the slider is mapped with the tapered profile.
    pub taper: bool,
    /// The attenuation at the bottom of the slider, in decibels.
    pub low_db: f64,
    /// The attenuation at the top of the slider, in decibels.
    pub high_db: f64,
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            codec: AudioCodecChoice::AacEld,
            latency_ms: 250,
            initial_volume: 100,
            taper: false,
            low_db: -30.0,
            high_db: 0.0,
        }
    }
}

impl AudioSettings {
    /// The largest delay the protocol accepts, in milliseconds.
    ///
    /// The receiver validates the value before it binds a socket, so a delay past this
    /// bound is a session that never starts rather than one that starts late.
    pub const MAX_LATENCY_MS: u32 = 10_000;

    /// The coding a session falls back to.
    pub fn codec(&self) -> rust_airplay::AudioCodec {
        match self.codec {
            AudioCodecChoice::Alac => rust_airplay::AudioCodec::Alac,
            AudioCodecChoice::AacEld => rust_airplay::AudioCodec::AacEld,
        }
    }

    /// How a slider position becomes a gain.
    pub fn volume_curve(&self) -> rust_airplay::VolumeCurve {
        rust_airplay::VolumeCurve { taper: self.taper, low_db: self.low_db, high_db: self.high_db }
    }

    /// The level a session starts at.
    pub fn initial_volume(&self) -> rust_airplay::Volume {
        rust_airplay::Volume::new(f64::from(self.initial_volume.min(100)) / 100.0)
    }

    /// The reported delay.
    pub fn latency(&self) -> std::time::Duration {
        std::time::Duration::from_millis(u64::from(self.latency_ms))
    }

    /// The problems with these values, if any.
    pub fn problems(&self) -> Vec<Problem> {
        let mut problems = Vec::new();
        if self.latency_ms > Self::MAX_LATENCY_MS {
            problems.push(Problem::new(
                "audio",
                "latency_ms",
                format!("must be at most {} milliseconds", Self::MAX_LATENCY_MS),
            ));
        }
        if self.initial_volume > 100 {
            problems.push(Problem::new("audio", "initial_volume", "must be a percentage"));
        }
        if self.low_db > self.high_db || self.low_db.is_nan() || self.high_db.is_nan() {
            problems.push(Problem::new(
                "audio",
                "low_db",
                "the bottom of the slider cannot be louder than the top",
            ));
        }
        if !(-90.0..=0.0).contains(&self.low_db) || !(-90.0..=0.0).contains(&self.high_db) {
            problems.push(Problem::new("audio", "high_db", "decibels are at most zero"));
        }
        problems
    }
}
