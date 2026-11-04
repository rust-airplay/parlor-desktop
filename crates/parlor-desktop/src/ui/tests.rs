// SPDX-License-Identifier: GPL-3.0-or-later
//! Clicks, through the real components.
//!
//! These tests run the window's own elements in a headless test window: a click goes
//! through GPUI's input path, the element's handler runs, and the settings document is what
//! it wrote. They exist because a control can compile, render and still hand its writer the
//! wrong value; only a click tells the difference.

use gpui_kit::component::setting::{
    SettingField, SettingGroup, SettingItem, SettingPage, Settings,
};
use gpui_kit::test::TestWindowExt as _;
use gpui_kit::{
    AnyWindowHandle, AppContext as _, BorrowAppContext as _, Context, Entity, Global, IntoElement,
    ParentElement as _, Render, SharedString, Styled as _, TestAppContext, Window, WindowOptions,
    div,
};

use crate::settings::Settings as Document;
use crate::state::Parlor;

/// A value the probe components write into, so a click can be observed without the real
/// settings pages.
#[derive(Default)]
struct Probe {
    theme: String,
    enabled: bool,
}

impl Global for Probe {}

/// One dropdown and one switch, built exactly the way the settings pages build theirs.
struct ProbeView;

impl Render for ProbeView {
    fn render(&mut self, _: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let pages = vec![SettingPage::new("Probe").group(SettingGroup::new().items(vec![
            SettingItem::new(
                "Theme",
                SettingField::dropdown(
                    vec![("system".into(), "System".into()), ("light".into(), "Light".into())],
                    |cx| SharedString::from(cx.global::<Probe>().theme.clone()),
                    |value: SharedString, cx| {
                        cx.update_global::<Probe, _>(|probe, _| probe.theme = value.to_string());
                    },
                ),
            ),
            SettingItem::new(
                "Enabled",
                SettingField::switch(
                    |cx| cx.global::<Probe>().enabled,
                    |value, cx| {
                        cx.update_global::<Probe, _>(|probe, _| probe.enabled = value);
                    },
                ),
            ),
        ]))];
        div().size_full().child(Settings::new("probe-settings").pages(pages))
    }
}

/// The window every click test runs in.
fn open_probe(cx: &mut TestAppContext) -> AnyWindowHandle {
    cx.update(gpui_kit::init);
    cx.update(|cx| cx.set_global(Probe::default()));
    let (handle, _view) = cx
        .update(|cx| {
            gpui_kit::open_window(WindowOptions::default(), cx, |_, cx| cx.new(|_| ProbeView))
        })
        .expect("a test window");
    handle
}

#[test]
fn a_dropdown_click_hands_the_writer_the_value() {
    let mut context = TestAppContext::single();
    let cx = &mut context;
    let handle = open_probe(cx);
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        // The Appearance page's theme field is this dropdown: open it, then choose the
        // second option. A field that handed back the label would write "Light".
        window.click("btn", cx);
        window.render_frame(cx);
        window.within("popup-menu").click(1usize, cx);
        window.render_frame(cx);
    })
    .expect("a frame");
    cx.update(|cx| assert_eq!(cx.global::<Probe>().theme, "light"));
}

#[test]
fn a_switch_click_hands_the_writer_the_new_state() {
    let mut context = TestAppContext::single();
    let cx = &mut context;
    let handle = open_probe(cx);
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        window.click("check", cx);
        window.render_frame(cx);
    })
    .expect("a frame");
    cx.update(|cx| assert!(cx.global::<Probe>().enabled, "the switch wrote its new state"));
}

/// A receiver window whose settings are the real ones, with two backends that a click test
/// does not need.
fn open_home(
    cx: &mut TestAppContext,
    paths: &crate::settings::store::Paths,
) -> (AnyWindowHandle, Entity<super::RootView>) {
    cx.update(gpui_kit::init);
    let document = Document::new();
    let identity = crate::settings::identity::Identity::test_identity([5; 32]);
    let backends = crate::render::detect::Backends {
        gstreamer: crate::render::detect::Availability::Missing {
            part: crate::render::detect::MissingPart::Runtime,
            reason: "a test".to_owned(),
        },
        ffmpeg: crate::render::detect::Availability::Ready {
            version: "test".to_owned(),
            location: std::path::PathBuf::from("ffmpeg"),
        },
    };
    let (installs, requests) = async_channel::unbounded();
    cx.update(move |cx| {
        cx.set_global(Parlor {
            settings: document.clone(),
            identity,
            paths: paths.clone(),
            backends,
            problems: Vec::new(),
            log: crate::session::logging::LogHandle::detached(),
            installs,
        });
        gpui_kit::open_window(WindowOptions::default(), cx, move |window, cx| {
            cx.new(|cx| super::RootView::new(window, cx, requests))
        })
        .expect("a test window")
    })
}

#[test]
fn a_home_choice_writes_the_document() {
    let mut context = TestAppContext::single();
    let cx = &mut context;
    let directory = tempfile::tempdir().expect("a temporary directory");
    let paths = crate::settings::store::Paths::at(directory.path());
    let (handle, _view) = open_home(cx, &paths);
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
        window.click("resolution-1280x720", cx);
        window.click("frame-rate-50", cx);
        window.click("backend-ffmpeg", cx);
        window.render_frame(cx);
    })
    .expect("a frame");
    cx.update(|cx| {
        let settings = &cx.global::<Parlor>().settings;
        assert_eq!((settings.display.width, settings.display.height), (1280, 720));
        assert_eq!(settings.display.max_fps, 50);
        assert_eq!(settings.backend.kind, crate::settings::BackendKind::Ffmpeg);
    });
    // The document on disk is what the window showed.
    let saved = crate::settings::store::load(&crate::settings::store::Paths::at(directory.path()))
        .expect("the written document");
    assert_eq!((saved.display.width, saved.display.height), (1280, 720));
    assert_eq!(saved.display.max_fps, 50);
    assert_eq!(saved.backend.kind, crate::settings::BackendKind::Ffmpeg);
}
#[test]
fn the_settings_page_renders() {
    let mut context = TestAppContext::single();
    let cx = &mut context;
    let directory = tempfile::tempdir().expect("a temporary directory");
    let paths = crate::settings::store::Paths::at(directory.path());
    let (handle, view) = open_home(cx, &paths);
    // The rail is styled markup rather than a component with an observed id, so the page is
    // selected directly; the frame that follows is what a click would have produced.
    cx.update(|cx| {
        view.update(cx, |view, cx| {
            view.tab = super::Tab::Settings;
            cx.notify();
        });
    });
    cx.update_window(handle, |_, window, cx| {
        window.render_frame(cx);
    })
    .expect("a frame");
    cx.update(|cx| assert_eq!(view.read(cx).tab, super::Tab::Settings));
}
