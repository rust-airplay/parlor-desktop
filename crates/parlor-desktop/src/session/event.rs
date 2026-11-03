// SPDX-License-Identifier: GPL-3.0-or-later
//! What a running receiver tells the window, and how it says it.
//!
//! Every callback the library offers is called on a thread the receiver owns. The window
//! is on another thread, and GPUI state may only be touched from its own. So the callbacks
//! do one thing: put a value on this channel. The window reads the channel and updates
//! itself there.

use crate::settings::store::Paths;

/// One thing that happened in a session.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// The thread is up and the backend is being built.
    Starting,
    /// The receiver bound its control port and is serving.
    Started {
        /// The port that was asked for. Port 0 asks the system for one, and the answer is
        /// in the log rather than here.
        port: u16,
    },
    /// The receiver stopped, because it was asked to or because it failed.
    Stopped,
    /// The receiver could not start, or stopped because of an error.
    Failed(String),
    /// A device is being asked for this PIN.
    Pin(String),
    /// A device was verified for the first time in this session.
    Client([u8; 32]),
    /// A device sent the metadata listing for what it is playing.
    Metadata(Vec<String>),
    /// A device sent artwork, which is written where the caller asked for it.
    Artwork {
        /// The content type the device gave, such as "image/jpeg".
        content_type: String,
        /// Where the bytes were written.
        path: std::path::PathBuf,
    },
    /// A device named the remote control connection it uses.
    RemoteControl {
        /// The DACP identifier.
        dacp_id: String,
        /// The Active-Remote header.
        active_remote: String,
    },
}

/// Writes artwork where a diagnosis can find it, under the application's own directory.
pub fn write_artwork(
    paths: &Paths,
    content_type: &str,
    bytes: &[u8],
) -> Option<std::path::PathBuf> {
    let extension = match content_type.split('/').next_back() {
        Some("jpeg" | "jpg") => "jpg",
        Some("png") => "png",
        _ => "img",
    };
    let path = paths.logs().join(format!("artwork.{extension}"));
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok()?;
    }
    std::fs::write(&path, bytes).ok()?;
    Some(path)
}
