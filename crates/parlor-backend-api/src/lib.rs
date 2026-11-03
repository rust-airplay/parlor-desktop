// SPDX-License-Identifier: GPL-3.0-or-later
//! The contract between the Parlor application and a renderer plugin.
//!
//! A playback backend that links a native library cannot be part of the application binary:
//! the binary would fail to start on a machine without that library, before any window could
//! offer to install it. Such a backend is compiled as a `cdylib` instead and loaded when a
//! session asks for it, which is why this crate exists. It names the symbols a plugin exports
//! and the JSON document the application passes to one of them.
//!
//! # The three symbols
//!
//! | Symbol | Signature | Meaning |
//! | --- | --- | --- |
//! | `parlor_backend_abi` | `extern "C" fn() -> u32` | the ABI version the plugin was built for |
//! | `parlor_backend_create` | `extern "C" fn(*const u8, usize) -> *mut c_void` | a renderer, or null |
//! | `parlor_backend_last_error` | `extern "C" fn() -> *const c_char` | why the last create returned null |
//!
//! The create call receives a UTF-8 JSON document holding [`GStreamerOptions`] and returns
//! either a pointer to a leaked `Box<Arc<dyn Renderer>>` or null. A null return is explained
//! by the error symbol, which reports a pointer to the last message; it is valid until the
//! next create call.
//!
//! # Why a raw pointer is sound here
//!
//! The plugin and the application are built in one workspace, from the same compiler and the
//! same `rust-airplay`, so the two copies of the `Renderer` trait agree. The pointer crosses
//! the seam as a boxed `Arc`, which is a sized value: the boxing is what keeps the pointer
//! thin and the cast to `*mut c_void` legal. Both sides share one C runtime heap, so the
//! application may drop the value the plugin allocated.

use std::ffi::c_void;

use serde::{Deserialize, Serialize};

/// The ABI version this crate describes.
///
/// A plugin built against another one is refused by name rather than called through a
/// pointer whose type the two sides disagree about.
pub const ABI_VERSION: u32 = 1;

/// The symbol that reports [`ABI_VERSION`].
pub const ABI_SYMBOL: &[u8] = b"parlor_backend_abi\0";

/// The symbol that creates a renderer from a [`GStreamerOptions`] document.
pub const CREATE_SYMBOL: &[u8] = b"parlor_backend_create\0";

/// The symbol that reports the last failure as a null-terminated UTF-8 string.
pub const ERROR_SYMBOL: &[u8] = b"parlor_backend_last_error\0";

/// The file name the GStreamer plugin is distributed under.
///
/// One name per platform, so a loader does not have to know which of the three it is
/// running on.
#[must_use]
pub fn plugin_file_name() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "parlor_gst_backend.dll"
    }
    #[cfg(target_os = "macos")]
    {
        "libparlor_gst_backend.dylib"
    }
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    {
        "libparlor_gst_backend.so"
    }
}

/// The GStreamer choices a session can make.
///
/// An empty field means "let the backend choose". That is not the same as naming a
/// value: the backend picks the first sink the machine actually has, which is a question
/// about the machine rather than about the session.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct GStreamerOptions {
    /// The audio sink, as a pipeline fragment such as `wasapisink`.
    pub audio_sink: String,
    /// The video sink, as a pipeline fragment such as `d3d11videosink`.
    pub video_sink: String,
    /// The element that parses H.264 access units.
    pub h264_parser: String,
    /// The element that decodes H.264 access units.
    pub h264_decoder: String,
    /// The element that parses H.265 access units.
    pub h265_parser: String,
    /// The element that decodes H.265 access units.
    pub h265_decoder: String,
    /// Whether the video pipeline is kept between sessions.
    pub warm_video: bool,
    /// Whether the backend honours a spacing asked for through `set_pace`.
    pub pacing: bool,
}

/// The type of [`ABI_SYMBOL`].
pub type AbiFn = unsafe extern "C" fn() -> u32;

/// The type of [`CREATE_SYMBOL`].
pub type CreateFn = unsafe extern "C" fn(*const u8, usize) -> *mut c_void;

/// The type of [`ERROR_SYMBOL`].
pub type ErrorFn = unsafe extern "C" fn() -> *const std::ffi::c_char;

#[cfg(test)]
mod tests {
    use super::{ABI_VERSION, GStreamerOptions};

    #[test]
    fn the_symbols_are_null_terminated() {
        assert_eq!(super::ABI_SYMBOL.last(), Some(&0));
        assert_eq!(super::CREATE_SYMBOL.last(), Some(&0));
        assert_eq!(super::ERROR_SYMBOL.last(), Some(&0));
    }

    #[test]
    fn an_empty_document_holds_the_defaults() {
        let options: GStreamerOptions = serde_json::from_str("{}").expect("an empty object");
        assert_eq!(options, GStreamerOptions::default());
        assert_eq!(ABI_VERSION, 1);
    }
}
