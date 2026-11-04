// SPDX-License-Identifier: GPL-3.0-or-later
//! The state every page reads: what was chosen, what this machine can do, and where the
//! files are.
//!
//! Settings are a global rather than a field of one view because the settings pages read
//! and write them from callbacks that only receive the whole application, and because the
//! home page must show the same values. A page that owned them would make every control a
//! channel to that page.

use gpui_kit::{App, BorrowAppContext as _, Global};

use crate::i18n::Lang;
use crate::render::detect::{self, Backends};
use crate::session::logging::LogHandle;
use crate::settings::Problem;
use crate::settings::Settings;
use crate::settings::identity::Identity;
use crate::settings::store::{self, Paths};

/// Everything the window shares.
pub struct Parlor {
    /// What the user chose.
    pub settings: Settings,
    /// The identity seed and the clients it has let in.
    pub identity: Identity,
    /// Where the application keeps its files.
    pub paths: Paths,
    /// What this machine can play through.
    pub backends: Backends,
    /// What is wrong with the settings, if anything.
    pub problems: Vec<Problem>,
    /// The records the diagnostics page shows.
    pub log: LogHandle,
    /// The channel the settings pages use to ask for an installation.
    pub installs: async_channel::Sender<crate::render::install::Target>,
}

impl Global for Parlor {}

/// The shared state.
pub fn parlor(cx: &App) -> &Parlor {
    cx.global::<Parlor>()
}

/// The language labels are written in.
pub fn lang(cx: &App) -> Lang {
    Lang::resolve(parlor(cx).settings.appearance.language)
}

/// Changes the settings, saves them, and shows the result.
///
/// Saving on every change is what makes the interface honest: a person who closes the
/// window has saved what the window showed, and there is no second thing to remember to
/// press.
pub fn update_settings(cx: &mut App, edit: impl FnOnce(&mut Settings)) {
    cx.update_global::<Parlor, _>(|parlor, cx| {
        edit(&mut parlor.settings);
        parlor.problems = parlor.settings.problems();
        if let Err(error) = store::save(&parlor.paths, &parlor.settings) {
            log::error!("cannot save the settings: {error}");
        }
        cx.refresh_windows();
    });
}

/// Changes the identity and writes it.
///
/// A seed that cannot be written is reported in the log and not shown as a dialog: the
/// session still runs, and a dialog about a file would be a dialog about the wrong thing.
pub fn update_identity(cx: &mut App, edit: impl FnOnce(&mut Identity)) {
    cx.update_global::<Parlor, _>(|parlor, cx| {
        edit(&mut parlor.identity);
        if let Err(error) = parlor.identity.save_clients(&parlor.paths) {
            log::error!("cannot save the remembered clients: {error}");
        }
        cx.refresh_windows();
    });
}

/// Looks for the backends again, after an installation or a removal.
pub fn reprobe(cx: &mut App) {
    cx.update_global::<Parlor, _>(|parlor, cx| {
        parlor.backends = detect::probe(&parlor.settings);
        log::info!(
            "reprobed: GStreamer {}, FFmpeg {}",
            describe(&parlor.backends.gstreamer),
            describe(&parlor.backends.ffmpeg)
        );
        cx.refresh_windows();
    });
}

/// One availability as a log line.
fn describe(availability: &detect::Availability) -> String {
    match availability {
        detect::Availability::Ready { version, .. } => format!("ready ({version})"),
        detect::Availability::Missing { part, reason } => format!("missing ({part:?}): {reason}"),
    }
}
