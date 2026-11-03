// SPDX-License-Identifier: GPL-3.0-or-later
//! What a client is told the receiver can show.
//!
//! The advertised feature bitmap is what a client reads before it decides how to encode. A
//! bit that is set offers a coding; a client that sends one the receiver did not offer is
//! not wrong, it is answering a different question.

use serde::{Deserialize, Serialize};

/// The optional codings a receiver can offer.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default)]
pub struct VideoSettings {
    /// Offer H.265 (HEVC) mirroring.
    ///
    /// A client that uses it sends roughly half the bytes of H.264 for the same picture.
    pub hevc: bool,
    /// Offer HTTP Live Streaming.
    pub hls: bool,
    /// Offer slideshow playback.
    pub slideshow: bool,
}

impl Default for VideoSettings {
    fn default() -> Self {
        Self { hevc: false, hls: false, slideshow: true }
    }
}

/// The bit that allows HEVC video, which the protocol's own receiver sets with -h265.
const HEVC_FEATURE: u64 = 1 << 42;
/// The bit that allows HTTP Live Streaming.
const HLS_FEATURE: u64 = 1 << 4;
/// The bit that allows slideshow playback.
const SLIDESHOW_FEATURE: u64 = 1 << 5;

impl VideoSettings {
    /// The bitmap to advertise.
    ///
    /// The protocol's defaults are the floor: a receiver that offered nothing would be
    /// answering a question no client asks.
    pub fn features(&self) -> u64 {
        let mut features = rust_airplay::server::DEFAULT_FEATURES;
        features &= !HEVC_FEATURE;
        features &= !HLS_FEATURE;
        features &= !SLIDESHOW_FEATURE;
        if self.hevc {
            features |= HEVC_FEATURE;
        }
        if self.hls {
            features |= HLS_FEATURE;
        }
        if self.slideshow {
            features |= SLIDESHOW_FEATURE;
        }
        features
    }
}

#[cfg(test)]
mod tests {
    use super::{HEVC_FEATURE, HLS_FEATURE, SLIDESHOW_FEATURE, VideoSettings};

    #[test]
    fn the_defaults_are_the_protocols_own_bitmap() {
        let features = VideoSettings::default().features();
        assert_eq!(features, rust_airplay::server::DEFAULT_FEATURES);
    }

    #[test]
    fn hevc_is_one_bit() {
        let features = VideoSettings { hevc: true, ..VideoSettings::default() }.features();
        assert_eq!(features & HEVC_FEATURE, HEVC_FEATURE);
        assert_eq!(features & HLS_FEATURE, 0);
    }

    #[test]
    fn slideshow_can_be_turned_off() {
        let features = VideoSettings { slideshow: false, ..VideoSettings::default() }.features();
        assert_eq!(features & SLIDESHOW_FEATURE, 0);
    }
}
