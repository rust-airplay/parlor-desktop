// SPDX-License-Identifier: GPL-3.0-or-later
//! The application: the files it reads first, and the window it opens.

use gpui_kit::component::{Theme, ThemeMode};
use gpui_kit::*;

use crate::session::logging::LogHandle;
use crate::settings::Settings;
use crate::settings::identity::Identity;
use crate::settings::store::{self, Paths};
use crate::state::Parlor;
use crate::ui::RootView;

/// Starts the application.
pub fn run() {
    let paths = Paths::discover().unwrap_or_else(|error| {
        // A machine with no per-user data directory cannot keep a setting. Starting with
        // the defaults in a temporary directory is better than refusing to start, and the
        // log says what happened as soon as there is a log to say it in.
        let fallback = std::env::temp_dir().join("Parlor");
        let _ = error;
        Paths::at(fallback)
    });
    let log_level = store::load(&paths)
        .map(|settings| settings.diagnostics.log_level.filter())
        .unwrap_or(log::LevelFilter::Info);
    let log = LogHandle::install(&paths, log_level).unwrap_or_else(|error| {
        // A logger that cannot be installed is still a reason to start: the records are
        // wanted, not required.
        let _ = error;
        LogHandle::install(&paths, log::LevelFilter::Off).expect("a logger")
    });
    let settings = store::load(&paths).unwrap_or_else(|error| {
        log::error!("{error}");
        Settings::new()
    });
    log.set_level(settings.diagnostics.log_level.filter());
    let identity = Identity::load(&paths).unwrap_or_else(|error| {
        log::error!("{error}");
        Identity::test_identity([0x42; 32])
    });
    let backends = crate::render::detect::probe(&settings);
    log::info!("GStreamer {}", describe(&backends.gstreamer));
    log::info!("FFmpeg    {}", describe(&backends.ffmpeg));
    let problems = settings.problems();
    let theme = settings.appearance.theme;

    gpui_kit::application().with_assets(gpui_kit::assets::Assets).run(move |cx| {
        gpui_kit::init(cx);
        cx.set_global(Parlor {
            settings: settings.clone(),
            identity,
            paths: paths.clone(),
            backends,
            problems,
            log,
        });
        apply_theme_without_window(theme, cx);
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::centered(size(px(1200.0), px(800.0)), cx)),
            window_min_size: Some(size(px(1000.0), px(640.0))),
            titlebar: Some(TitlebarOptions { title: Some("Parlor".into()), ..Default::default() }),
            app_id: Some("parlor".into()),
            ..Default::default()
        };
        gpui_kit::open_window(options, cx, move |window, cx| {
            cx.new(|cx| RootView::new(window, cx))
        })
        .expect("the window");
    });
}

/// One backend's availability, for the log.
fn describe(availability: &crate::render::detect::Availability) -> String {
    use crate::render::detect::Availability;
    match availability {
        Availability::Ready { version, location } => {
            format!("ready: {version} ({})", location.display())
        }
        Availability::Missing { part, reason } => format!("missing ({part:?}): {reason}"),
    }
}

/// Applies a theme before a window exists, for the one case that has no window yet.
fn apply_theme_without_window(choice: crate::settings::ThemeChoice, cx: &mut App) {
    match choice {
        crate::settings::ThemeChoice::Light => Theme::change(ThemeMode::Light, None, cx),
        crate::settings::ThemeChoice::Dark => Theme::change(ThemeMode::Dark, None, cx),
        // The window's own appearance decides, which RootView asks for once it exists.
        crate::settings::ThemeChoice::System => {}
    }
}
