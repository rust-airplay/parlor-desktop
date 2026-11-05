// SPDX-License-Identifier: GPL-3.0-or-later
//! Parlor: an AirPlay receiver with a desktop interface.
//!
//! The program is a window over [`rust_airplay`]: it collects the settings a person chooses,
//! starts a receiver with them, and shows what the receiver reports. The protocol lives in
//! the library and the playback in a backend; nothing here talks to a client.

mod app;
mod i18n;
mod render;
mod session;
mod settings;
mod state;
mod ui;
mod update;

fn main() {
    app::run();
}
