// SPDX-License-Identifier: GPL-3.0-or-later
//! The GStreamer renderer as a library the application loads at run time.
//!
//! The application does not link GStreamer. This crate does, which makes it the only part of
//! Parlor that needs the GStreamer development files to build and the GStreamer runtime to
//! load. Everything the application knows about that is `parlor_backend_api`: two exported
//! symbols and a JSON document describing the session's choices.
//!
//! The plugin holds no state between calls beyond the last error message. A renderer is
//! built, handed to the application, and dropped by it when the session ends; the GStreamer
//! objects a renderer owns are released with it.

#![allow(unsafe_code)] // The plugin boundary is this crate's reason to exist; see above.

use std::ffi::{CString, c_char, c_void};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{Arc, Mutex};

use parlor_backend_api::{ABI_VERSION, GStreamerOptions};
use rust_airplay::{Renderer, VideoCodec};
use rust_airplay_gstream::{GstRenderer, Options};

/// The last failure, for the application to read after a null pointer.
///
/// A renderer is created on the thread that starts a session, before the session runs, and
/// the pointer is read before the next create call. A plain mutex is therefore enough, and a
/// message that is overwritten is a message about a create call that already returned.
static LAST_ERROR: Mutex<Option<CString>> = Mutex::new(None);

/// Reports the ABI version this plugin was built for.
///
/// The application refuses a plugin whose answer is not its own version, so a stale plugin
/// beside a newer application is a refusal with a message rather than a call through a
/// pointer the two sides disagree about.
#[unsafe(no_mangle)]
pub extern "C" fn parlor_backend_abi() -> u32 {
    ABI_VERSION
}

/// Reports the last failure as a null-terminated string.
///
/// The pointer is owned by the plugin and is valid until the next create call. A caller that
/// needs the text after another call copies it.
#[unsafe(no_mangle)]
pub extern "C" fn parlor_backend_last_error() -> *const c_char {
    match LAST_ERROR.lock() {
        Ok(slot) => slot.as_ref().map_or(std::ptr::null(), |text| text.as_ptr()),
        Err(_) => std::ptr::null(),
    }
}

/// Creates a renderer from a JSON document, or returns null.
///
/// # Safety
///
/// `json` must point to `len` readable bytes for the duration of the call. The returned
/// pointer is a `Box<Arc<dyn Renderer>>` that the caller owns; it must be passed to
/// `Box::from_raw` exactly once, in a program built against the same ABI.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn parlor_backend_create(json: *const u8, len: usize) -> *mut c_void {
    // A panic must not unwind into a caller that cannot catch it: the application would
    // lose the session instead of reading a message about it.
    let created = catch_unwind(AssertUnwindSafe(|| unsafe { create_renderer(json, len) }));
    match created {
        Ok(Ok(pointer)) => pointer,
        Ok(Err(message)) => {
            set_error(message);
            std::ptr::null_mut()
        }
        Err(_) => {
            set_error("the GStreamer backend panicked while creating a renderer".to_owned());
            std::ptr::null_mut()
        }
    }
}

/// The fallible part of [`parlor_backend_create`].
///
/// # Safety
///
/// The same contract as the exported function it serves.
unsafe fn create_renderer(json: *const u8, len: usize) -> Result<*mut c_void, String> {
    if json.is_null() && len > 0 {
        return Err("the options pointer is null".to_owned());
    }
    let bytes = if len == 0 {
        &[][..]
    } else {
        // SAFETY: the caller promises `len` readable bytes at `json` for this call.
        unsafe { std::slice::from_raw_parts(json, len) }
    };
    let options: GStreamerOptions = serde_json::from_slice(bytes)
        .map_err(|error| format!("cannot read the options: {error}"))?;
    let renderer = GstRenderer::with_options(gst_options(&options))
        .map_err(|error| format!("the GStreamer runtime refused the options: {error}"))?;
    let renderer: Arc<dyn Renderer> = Arc::new(renderer);
    // A `Box<Arc<..>>` is a thin pointer, which is what an FFI seam can carry. The caller
    // takes the value back with `Box::from_raw` and drops the Arc when the session ends.
    let boxed: Box<Arc<dyn Renderer>> = Box::new(renderer);
    Ok(Box::into_raw(boxed).cast::<c_void>())
}

/// Maps the session's choices onto the backend's options.
///
/// A field the session left empty is a question the backend answers: it picks the first sink
/// the machine actually has. That is why a sink is applied only when it names something.
fn gst_options(options: &GStreamerOptions) -> Options {
    let mut gst = Options::default();
    if !options.audio_sink.trim().is_empty() {
        gst = gst.with_audio_sink(options.audio_sink.trim());
    }
    if !options.video_sink.trim().is_empty() {
        gst = gst.with_video_sink(options.video_sink.trim());
    }
    if !options.h264_parser.trim().is_empty() || !options.h264_decoder.trim().is_empty() {
        gst = gst.with_video_chain(
            VideoCodec::H264,
            or_default(&options.h264_parser, "h264parse"),
            or_default(&options.h264_decoder, "avdec_h264"),
        );
    }
    if !options.h265_parser.trim().is_empty() || !options.h265_decoder.trim().is_empty() {
        gst = gst.with_video_chain(
            VideoCodec::H265,
            or_default(&options.h265_parser, "h265parse"),
            or_default(&options.h265_decoder, "avdec_h265"),
        );
    }
    gst.with_warm_video(options.warm_video).with_pacing(options.pacing)
}

/// The value the session named, or the default one.
fn or_default(named: &str, fallback: &str) -> String {
    let named = named.trim();
    if named.is_empty() { fallback.to_owned() } else { named.to_owned() }
}

/// Records a failure for [`parlor_backend_last_error`].
fn set_error(message: String) {
    // A message is recorded even when the lock is poisoned: the mutex protects a
    // diagnostic, and losing the diagnostic because something else panicked makes the
    // panic harder to find.
    let slot = match LAST_ERROR.lock() {
        Ok(slot) => slot,
        Err(poisoned) => poisoned.into_inner(),
    };
    drop(slot);
    let mut slot = match LAST_ERROR.lock() {
        Ok(slot) => slot,
        Err(poisoned) => poisoned.into_inner(),
    };
    *slot = Some(CString::new(message).unwrap_or_default());
}
