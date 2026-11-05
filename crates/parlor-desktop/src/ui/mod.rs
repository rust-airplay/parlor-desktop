// SPDX-License-Identifier: GPL-3.0-or-later
//! The window: a rail, a page, and the session the two of them talk about.
//!
//! One view owns the running session, because a session is not a setting: its state is
//! what the receiver reports while the window is open, and the window is the only thing
//! that shows it. The settings are a global (see [state](crate::state)) because every
//! control on the settings pages reads and writes them through callbacks that receive the
//! whole application.

pub mod home;
pub mod pages;

#[cfg(test)]
mod tests;

use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Receiver as Channel;
use std::time::Duration;

use async_channel::Receiver as Requests;
use gpui_kit::component::ActiveTheme as _;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::component::{Icon, IconName, Sizable, Theme, WindowExt, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::i18n::Text;
use crate::render::install::{Installer, Progress, Target};
use crate::session::Session;
use crate::session::event::Event;
use crate::settings::Settings;
use crate::settings::identity;
use crate::state;

/// Which page the rail has selected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    /// The receiver, its PIN, and the picture settings used while it runs.
    Home,
    /// Everything else.
    Settings,
}

/// What the receiver is doing, as far as the window knows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    /// Not running.
    Stopped,
    /// The thread exists; the receiver has not reported a port yet.
    Starting,
    /// Serving on this port.
    Running(u16),
    /// Asked to stop, waiting for the thread to end.
    Stopping,
    /// Could not start, or stopped with an error.
    Failed(String),
}

/// The channels behind the about page's update check.
pub struct Updates {
    /// Requests from the about page.
    pub(crate) requests: Requests<()>,
    /// Answers from the thread that asked.
    pub(crate) results: Requests<crate::update::UpdateState>,
    /// The thread's end of those answers.
    pub(crate) sender: async_channel::Sender<crate::update::UpdateState>,
}

/// One installation in flight.
struct Installation {
    target: Target,
    progress: Progress,
    cancel: Arc<AtomicBool>,
    events: Channel<Progress>,
}

/// The window's content.
pub struct RootView {
    /// The selected page.
    pub tab: Tab,
    /// The running receiver, when there is one.
    pub session: Option<Session>,
    /// What the receiver last said it was doing.
    pub status: Status,
    /// The PIN a device is being asked for, while it is being asked.
    pub pin: Option<String>,
    /// The devices that have connected since the window opened.
    pub clients: Vec<String>,
    /// What the device says it is playing.
    pub now_playing: Vec<String>,
    /// Recent lines for the home page's activity list.
    pub activity: Vec<String>,
    /// Installations the settings pages asked for.
    requests: Requests<Target>,
    /// The installation in flight, if any.
    installation: Option<Installation>,
    /// The observer that follows the operating system's appearance.
    appearance: Option<Subscription>,
    /// The document the running receiver was started from, for the pending-restart notice.
    applied: Option<Settings>,
    /// Whether the receiver was asked to stop in order to start again.
    restart_pending: bool,
    /// Whether the first-run dialog is open.
    asked_for_backend: bool,
    /// The update check's channels.
    updates: Updates,
    /// The session name, edited on the home page.
    pub name_input: Entity<InputState>,
    /// Keeps the name the input shows and the name the settings hold together.
    _name_subscription: Subscription,
}

impl RootView {
    /// Builds the view for a window, and starts reading what the receiver says.
    pub fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        requests: Requests<Target>,
        updates: Updates,
    ) -> Self {
        let follows_system =
            state::parlor(cx).settings.appearance.theme == crate::settings::ThemeChoice::System;
        let name = state::parlor(cx).settings.receiver.name.clone();
        let name_input = cx.new(|cx| InputState::new(window, cx).default_value(name));
        let name_subscription = cx.subscribe(&name_input, |_, input, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                let value = input.read(cx).value().to_string();
                state::update_settings(cx, |settings| settings.receiver.name = value);
            }
        });
        let mut view = Self {
            tab: Tab::Home,
            session: None,
            status: Status::Stopped,
            pin: None,
            clients: Vec::new(),
            now_playing: Vec::new(),
            activity: Vec::new(),
            requests,
            installation: None,
            appearance: None,
            applied: None,
            restart_pending: false,
            asked_for_backend: false,
            updates,
            name_input,
            _name_subscription: name_subscription,
        };
        if follows_system {
            Theme::sync_system_appearance(Some(window), cx);
        }
        // The appearance can change while the window is open, and the choice can change to
        // "follow the system" while it is open. One observer answers both: it re-applies the
        // system appearance only while that is what the settings ask for.
        let subscription = cx.observe_window_appearance(window, |_, window, cx| {
            if state::parlor(cx).settings.appearance.theme == crate::settings::ThemeChoice::System {
                Theme::sync_system_appearance(Some(window), cx);
            }
        });
        view.appearance = Some(subscription);
        view.pump(cx);
        if !state::parlor(cx).backends.any_ready() {
            // The kit's Root owns the dialog layers and is mounted after this callback
            // returns, so opening the dialog from here would look for a window state that
            // does not exist yet. Deferring it to the first frame is what makes the
            // no-backend case a dialog instead of a panic.
            cx.defer_in(window, |view, window, cx| view.ask_for_backend(window, cx));
        }
        view
    }

    /// Asks the repository for the latest version, on a thread of its own so a slow
    /// network cannot stop the window drawing.
    fn check_for_update(&mut self, cx: &mut Context<Self>) {
        state::set_update(cx, crate::update::UpdateState::Checking);
        let sender = self.updates.sender.clone();
        std::thread::spawn(move || {
            let update = crate::update::check();
            let _ = sender.send_blocking(update);
        });
    }

    /// Reads the session and the installer, and repaints when either moved.
    fn pump(&mut self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(Duration::from_millis(150)).await;
                let alive = this.update(cx, |view, cx| view.drain(cx));
                if alive.is_err() {
                    break;
                }
            }
        })
        .detach();
    }

    /// Drains both channels once, and repaints if anything moved.
    fn drain(&mut self, cx: &mut Context<Self>) {
        let mut changed = false;
        let mut events = Vec::new();
        if let Some(session) = &self.session {
            while let Ok(event) = session.events().try_recv() {
                events.push(event);
            }
        }
        for event in events {
            self.apply(event, cx);
            changed = true;
        }
        let mut finished = false;
        if let Some(installation) = &mut self.installation {
            while let Ok(progress) = installation.events.try_recv() {
                installation.progress = progress;
                changed = true;
            }
            finished = installation.progress.is_final();
        }
        if finished {
            self.installation = None;
            state::reprobe(cx);
            changed = true;
            // A backend that just arrived answers the first-run dialog's question, so the
            // dialog should not stay in front of the window telling the person to install it.
            if self.asked_for_backend && state::parlor(cx).backends.any_ready() {
                self.asked_for_backend = false;
                for handle in cx.windows() {
                    let _ = cx.update_window(handle, |_, window, cx| {
                        if window.has_active_dialog(cx) {
                            window.close_dialog(cx);
                        }
                    });
                }
            }
        }
        let mut checks = 0;
        while self.updates.requests.try_recv().is_ok() {
            checks += 1;
        }
        for _ in 0..checks {
            self.check_for_update(cx);
        }
        while let Ok(update) = self.updates.results.try_recv() {
            state::set_update(cx, update);
            changed = true;
        }
        let mut requested = Vec::new();
        while let Ok(target) = self.requests.try_recv() {
            requested.push(target);
        }
        for target in requested {
            self.install(target, cx);
            changed = true;
        }
        if changed {
            cx.notify();
        }
    }

    /// Applies one thing the receiver said.
    fn apply(&mut self, event: Event, cx: &mut Context<Self>) {
        match event {
            Event::Starting => self.status = Status::Starting,
            Event::Started { port } => self.status = Status::Running(port),
            Event::Stopped => {
                self.status = Status::Stopped;
                self.session = None;
                self.pin = None;
                if self.restart_pending {
                    self.restart_pending = false;
                    self.start(cx);
                }
            }
            Event::Failed(reason) => {
                self.status = Status::Failed(reason.clone());
                self.session = None;
                self.pin = None;
                self.remember(reason);
            }
            Event::Pin(pin) => self.pin = Some(pin),
            Event::Client(key) => {
                let hex = identity::hex(&key);
                if !self.clients.contains(&hex) {
                    self.clients.push(hex.clone());
                }
                if state::parlor(cx).settings.auth.remember_clients {
                    state::update_identity(cx, |identity| {
                        identity.remember(key);
                    });
                }
                self.remember(hex);
            }
            Event::Metadata(lines) => self.now_playing = lines,
            Event::Artwork { content_type, path } => {
                self.remember(format!("{content_type} {}", path.display()));
            }
            Event::RemoteControl { dacp_id, active_remote } => {
                self.remember(format!("DACP {dacp_id} {active_remote}"));
            }
        }
    }

    /// Adds one line to the activity list, keeping the newest few.
    fn remember(&mut self, line: String) {
        self.activity.push(line);
        if self.activity.len() > 8 {
            self.activity.remove(0);
        }
    }

    /// Starts a receiver with the current settings.
    pub fn start(&mut self, cx: &mut Context<Self>) {
        if self.session.is_some() {
            return;
        }
        let parlor = state::parlor(cx);
        if let Some(problem) = parlor.problems.first() {
            self.status = Status::Failed(format!("{}: {}", problem.field, problem.message));
            cx.notify();
            return;
        }
        match Session::start(
            parlor.settings.clone(),
            parlor.identity.clone(),
            parlor.paths.clone(),
            parlor.log.clone(),
        ) {
            Ok(session) => {
                self.session = Some(session);
                self.status = Status::Starting;
                self.pin = None;
                self.applied = Some(parlor.settings.clone());
            }
            Err(error) => self.status = Status::Failed(error),
        }
        cx.notify();
    }

    /// Whether the settings differ from the ones the running receiver was started with.
    pub fn has_pending(&self, cx: &App) -> bool {
        let Some(applied) = &self.applied else {
            return false;
        };
        self.session.is_some() && applied.session_differs(&state::parlor(cx).settings)
    }

    /// Stops and starts the receiver, which is how a change takes effect.
    pub fn restart(&mut self, cx: &mut Context<Self>) {
        if self.session.is_some() {
            self.restart_pending = true;
            self.stop(cx);
        } else {
            self.start(cx);
        }
    }

    /// Asks the receiver to stop.
    pub fn stop(&mut self, cx: &mut Context<Self>) {
        if let Some(session) = &mut self.session {
            session.stop();
            self.status = Status::Stopping;
        }
        cx.notify();
    }

    /// Starts one installation on a thread of its own.
    fn install(&mut self, target: Target, cx: &mut Context<Self>) {
        if self.installation.is_some() {
            return;
        }
        let paths = state::parlor(cx).paths.clone();
        let installer = Installer::new(target, paths);
        let cancel = installer.cancel_handle();
        let (sender, events) = std::sync::mpsc::channel();
        std::thread::Builder::new()
            .name("parlor-install".to_owned())
            .spawn(move || {
                let mut report = |progress: Progress| {
                    let _ = sender.send(progress);
                };
                installer.install(&mut report);
            })
            .ok();
        self.installation =
            Some(Installation { target, progress: Progress::Preparing, cancel, events });
    }

    /// The installation in flight, for a page that wants to show it.
    pub fn installation(&self) -> Option<(&Target, &Progress)> {
        self.installation
            .as_ref()
            .map(|installation| (&installation.target, &installation.progress))
    }

    /// Asks for an installation to be stopped.
    pub fn cancel_installation(&mut self, cx: &mut Context<Self>) {
        if let Some(installation) = &self.installation {
            installation.cancel.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        cx.notify();
    }

    /// The first-run dialog, when this machine has nothing to play through.
    fn ask_for_backend(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.asked_for_backend = true;
        let lang = state::lang(cx);
        let text = move |text: Text| text.of(lang);
        window.open_dialog(cx, move |dialog, _, _| {
            dialog
                .title(text(Text::NoBackendDialogTitle))
                .child(
                    v_flex().gap_3().child(text(Text::NoBackendDialogBody)).child(
                        h_flex()
                            .gap_2()
                            .child(
                                Button::new("install-ffmpeg")
                                    .primary()
                                    .label(text(Text::InstallFfmpeg))
                                    .on_click(|_, _, cx| {
                                        let _ = state::parlor(cx).installs.try_send(Target::Ffmpeg);
                                    }),
                            )
                            .child(
                                Button::new("install-gstreamer")
                                    .label(text(Text::InstallGStreamer))
                                    .on_click(|_, _, cx| {
                                        let _ =
                                            state::parlor(cx).installs.try_send(Target::GStreamer);
                                    }),
                            ),
                    ),
                )
                .footer(
                    h_flex()
                        .gap_2()
                        .child(Button::new("continue").label(text(Text::ContinueWithout)).on_click(
                            |_, window, cx| {
                                window.close_dialog(cx);
                            },
                        ))
                        .child(Button::new("quit").label(text(Text::Quit)).on_click(|_, _, cx| {
                            cx.quit();
                        })),
                )
        });
    }

    /// The rail: the pages, and what the receiver is doing.
    fn rail(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let lang = state::lang(cx);
        let text = move |text: Text| text.of(lang);
        let theme = cx.theme();
        let background = theme.sidebar;
        let border = theme.border;
        let muted = theme.muted_foreground;
        let active = theme.accent;
        let active_text = theme.accent_foreground;
        let radius = theme.radius;
        let dot_and_label = self.status_style(cx);
        let (dot, status_label) = dot_and_label;
        let pages = [
            (Tab::Home, IconName::LayoutDashboard, text(Text::NavHome)),
            (Tab::Settings, IconName::Settings, text(Text::NavSettings)),
        ];
        v_flex()
            .w(px(208.0))
            .h_full()
            .flex_shrink_0()
            .p_3()
            .gap_1()
            .bg(background)
            .border_r_1()
            .border_color(border)
            .child(
                h_flex()
                    .px_2()
                    .py_3()
                    .gap_2()
                    .items_center()
                    .child(div().text_lg().font_weight(gpui::FontWeight::BOLD).child("Parlor")),
            )
            .children(pages.into_iter().enumerate().map(|(ix, (tab, icon, label))| {
                let selected = self.tab == tab;
                h_flex()
                    .id(SharedString::from(format!("rail-{ix}")))
                    .gap_2()
                    .items_center()
                    .px_2()
                    .py_2()
                    .rounded(radius)
                    .when(selected, |this| this.bg(active).text_color(active_text))
                    .when(!selected, |this| this.text_color(muted))
                    .cursor_pointer()
                    .child(Icon::new(icon).small())
                    .child(label)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.tab = tab;
                        cx.notify();
                    }))
            }))
            .child(div().flex_1())
            .child(
                v_flex()
                    .px_2()
                    .py_2()
                    .gap_1()
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(div().size(px(8.0)).rounded_full().bg(dot))
                            .child(div().text_sm().child(status_label)),
                    )
                    .when(self.has_pending(cx), |this| {
                        this.child(
                            v_flex()
                                .gap_1()
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(muted)
                                        .child(text(Text::PendingBody)),
                                )
                                .child(
                                    Button::new("rail-restart")
                                        .xsmall()
                                        .label(text(Text::Restart))
                                        .on_click(cx.listener(|this, _, _, cx| this.restart(cx))),
                                ),
                        )
                    }),
            )
    }

    /// The colour and the label of the status the rail shows.
    fn status_style(&self, cx: &App) -> (gpui::Hsla, &'static str) {
        let theme = cx.theme();
        let lang = state::lang(cx);
        match &self.status {
            Status::Stopped => (theme.muted_foreground, Text::StatusStopped.of(lang)),
            Status::Starting => (theme.warning, Text::StatusStarting.of(lang)),
            Status::Running(_) => (theme.success, Text::StatusRunning.of(lang)),
            Status::Stopping => (theme.warning, Text::StatusStopping.of(lang)),
            Status::Failed(_) => (theme.danger, Text::StatusFailed.of(lang)),
        }
    }
}

impl Render for RootView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let background = theme.background;
        let foreground = theme.foreground;
        let page = match self.tab {
            Tab::Home => home::render(self, window, cx).into_any_element(),
            Tab::Settings => pages::render(self, window, cx).into_any_element(),
        };
        h_flex()
            .size_full()
            .bg(background)
            .text_color(foreground)
            .child(self.rail(window, cx))
            .child(div().h_full().flex_1().min_w_0().child(page))
    }
}
