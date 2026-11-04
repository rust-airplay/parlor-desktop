// SPDX-License-Identifier: GPL-3.0-or-later
//! The home page: the receiver, the picture, and what it is doing.
//!
//! The controls a person reaches for while watching are here - start and stop, the size of
//! the picture, the frame rate, the backend - and everything they configure once is on the
//! settings pages.

use gpui_kit::component::ActiveTheme as _;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::progress::Progress as ProgressBar;
use gpui_kit::component::scroll::ScrollableElement as _;
use gpui_kit::component::{Sizable, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::i18n::Text;
use crate::render::detect::Availability;
use crate::render::install::{Progress, Target};
use crate::settings::{BackendKind, PRESETS};
use crate::state;
use crate::ui::{RootView, Status};

/// The page.
pub fn render(
    view: &mut RootView,
    _window: &mut Window,
    cx: &mut Context<RootView>,
) -> impl IntoElement {
    let lang = state::lang(cx);
    let text = move |label: Text| label.of(lang);
    let theme = cx.theme();
    let background = theme.background;
    let border = theme.border;
    let muted = theme.muted_foreground;
    let radius = theme.radius;
    let parlor = state::parlor(cx);
    let settings = parlor.settings.clone();
    let backends = parlor.backends.clone();
    let problems = parlor.problems.clone();
    let remembered = parlor.identity.clients().to_vec();
    let status = view.status.clone();
    let pin = view.pin.clone();
    let activity = view.activity.clone();
    let now_playing = view.now_playing.clone();
    let installation = view.installation().map(|(target, progress)| (*target, progress.clone()));
    let running = view.session.is_some();

    let mut column = v_flex().p_6().gap_4().w_full().max_w(px(900.0));

    // The heading.
    column = column.child(
        v_flex()
            .gap_1()
            .child(div().text_xl().font_weight(FontWeight::BOLD).child(text(Text::HomeTitle)))
            .child(div().text_sm().text_color(muted).child(text(Text::HomeSubtitle))),
    );

    // The receiver, and the one button that matters while watching.
    let card =
        || v_flex().gap_3().p_4().rounded(radius).border_1().border_color(border).bg(background);
    let mut session = card().child(
        h_flex()
            .justify_between()
            .items_center()
            .child(
                v_flex()
                    .gap_1()
                    .child(
                        h_flex().gap_2().items_center().child(status_dot(&status, cx)).child(
                            div()
                                .text_base()
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(status_label(&status, cx)),
                        ),
                    )
                    .child(div().text_sm().text_color(muted).child(session_detail(
                        &status,
                        settings.receiver.name(),
                        settings.network.port,
                    ))),
            )
            .child(
                Button::new("toggle-session")
                    .primary()
                    .label(if running { text(Text::Stop) } else { text(Text::Start) })
                    .on_click(cx.listener(|this, _, _, cx| {
                        if this.session.is_some() {
                            this.stop(cx);
                        } else {
                            this.start(cx);
                        }
                    })),
            ),
    );
    if let Some(problem) = problems.first() {
        session = session.child(
            div()
                .text_sm()
                .text_color(cx.theme().danger)
                .child(format!("{}: {}", problem.field, problem.message)),
        );
    }
    column = column.child(session);

    // A change that a running receiver cannot see is said out loud, with the one button
    // that applies it.
    if view.has_pending(cx) {
        column = column.child(
            card()
                .child(section_title(text(Text::PendingTitle)))
                .child(div().text_sm().text_color(muted).child(text(Text::PendingBody)))
                .child(
                    Button::new("restart-session")
                        .primary()
                        .label(text(Text::Restart))
                        .on_click(cx.listener(|this, _, _, cx| this.restart(cx))),
                ),
        );
    }

    // The PIN, while a device is being asked for it.
    if let Some(pin) = &pin {
        column = column.child(
            card()
                .items_center()
                .child(div().text_sm().text_color(muted).child(text(Text::PinTitle)))
                .child(div().text_3xl().font_weight(FontWeight::BOLD).child(pin.clone()))
                .child(div().text_xs().text_color(muted).child(text(Text::PinHint))),
        );
    }

    // Nothing to play through: the two installers.
    if !backends.any_ready() {
        column = column.child(
            card()
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(text(Text::NoBackendTitle)),
                )
                .child(div().text_sm().text_color(muted).child(text(Text::NoBackendBody)))
                .child(
                    h_flex()
                        .gap_2()
                        .child(install_button(
                            "install-ffmpeg",
                            text(Text::InstallFfmpeg),
                            Target::Ffmpeg,
                        ))
                        .child(install_button(
                            "install-gstreamer",
                            text(Text::InstallGStreamer),
                            Target::GStreamer,
                        )),
                ),
        );
    }

    // An installation in flight.
    if let Some((target, progress)) = installation {
        let fraction = progress.fraction();
        column = column.child(
            card()
                .child(div().text_sm().font_weight(FontWeight::SEMIBOLD).child(match target {
                    Target::Ffmpeg => text(Text::Ffmpeg),
                    Target::GStreamer => text(Text::GStreamer),
                }))
                .child(ProgressBar::new("install").value(fraction as f32 * 100.0))
                .child(div().text_sm().text_color(muted).child(progress_label(&progress, text)))
                .child(
                    Button::new("cancel-install")
                        .label(text(Text::InstallCancel))
                        .on_click(cx.listener(|this, _, _, cx| this.cancel_installation(cx))),
                ),
        );
    }

    // The picture: the size and the rate are what a device is asked for.
    column = column.child(
        card()
            .child(section_title(text(Text::QualityTitle)))
            .child(quick_row(
                text(Text::Resolution),
                "resolution",
                PRESETS
                    .iter()
                    .map(|(width, height)| {
                        let selected =
                            settings.display.width == *width && settings.display.height == *height;
                        let (width, height) = (*width, *height);
                        QuickChoice {
                            label: format!("{width}x{height}"),
                            selected,
                            action: Box::new(move |cx: &mut App| {
                                state::update_settings(cx, |settings| {
                                    settings.display.width = width;
                                    settings.display.height = height;
                                });
                            }),
                        }
                    })
                    .collect(),
            ))
            .child(quick_row(
                text(Text::FrameRate),
                "frame-rate",
                crate::settings::DisplaySettings::FRAME_RATES
                    .iter()
                    .map(|fps| {
                        let selected = settings.display.max_fps == *fps;
                        let fps = *fps;
                        QuickChoice {
                            label: format!("{fps}"),
                            selected,
                            action: Box::new(move |cx: &mut App| {
                                state::update_settings(cx, |settings| {
                                    settings.display.max_fps = fps;
                                });
                            }),
                        }
                    })
                    .collect(),
            )),
    );

    // Which backend plays, and whether this machine has it.
    column = column.child(
        card().child(section_title(text(Text::Backend))).child(
            h_flex()
                .gap_2()
                .child(backend_button(
                    "backend-gstreamer",
                    text(Text::GStreamer),
                    settings.backend.kind == BackendKind::GStreamer,
                    &backends.gstreamer,
                    BackendKind::GStreamer,
                ))
                .child(backend_button(
                    "backend-ffmpeg",
                    text(Text::Ffmpeg),
                    settings.backend.kind == BackendKind::Ffmpeg,
                    &backends.ffmpeg,
                    BackendKind::Ffmpeg,
                )),
        ),
    );

    // What has connected, and what it is playing.
    let mut devices =
        card().child(section_title(text(Text::Devices))).child(if remembered.is_empty() {
            div().text_sm().text_color(muted).child(text(Text::NoDevices)).into_any_element()
        } else {
            v_flex()
                .gap_1()
                .children(
                    remembered
                        .iter()
                        .map(|key| {
                            div()
                                .text_xs()
                                .text_color(muted)
                                .child(short_key(&crate::settings::identity::hex(key)))
                        })
                        .collect::<Vec<_>>(),
                )
                .into_any_element()
        });
    if !now_playing.is_empty() {
        devices = devices.child(div().text_sm().child(now_playing.join("  ")));
    }
    column = column.child(devices);

    if !activity.is_empty() {
        column = column.child(
            card().child(section_title(text(Text::Activity))).child(
                v_flex().gap_1().children(
                    activity
                        .iter()
                        .map(|line| div().text_xs().text_color(muted).child(line.clone()))
                        .collect::<Vec<_>>(),
                ),
            ),
        );
    }

    v_flex().id("home").size_full().overflow_y_scrollbar().child(column)
}

/// One quick choice on the home page.
struct QuickChoice {
    label: String,
    selected: bool,
    action: Box<dyn Fn(&mut App)>,
}

/// One row of quick choices: a label and a line of buttons.
fn quick_row(
    label: &'static str,
    prefix: &'static str,
    choices: Vec<QuickChoice>,
) -> impl IntoElement {
    v_flex().gap_2().child(div().text_sm().child(label)).child(
        h_flex().gap_1().flex_wrap().children(choices.into_iter().map(|choice| {
            let id = SharedString::from(format!("{prefix}-{}", choice.label));
            Button::new(id)
                .small()
                .label(choice.label)
                .when(choice.selected, |this| this.primary())
                .on_click(move |_, _, cx| (choice.action)(cx))
        })),
    )
}

/// A card heading.
fn section_title(title: &'static str) -> impl IntoElement {
    div().text_sm().font_weight(FontWeight::SEMIBOLD).child(title)
}

/// The button that installs one backend.
fn install_button(id: &'static str, label: &'static str, target: Target) -> impl IntoElement {
    Button::new(id).label(label).on_click(move |_, _, cx| {
        let _ = state::parlor(cx).installs.try_send(target);
    })
}

/// The button that chooses a backend.
fn backend_button(
    id: &'static str,
    label: &'static str,
    selected: bool,
    availability: &Availability,
    kind: BackendKind,
) -> impl IntoElement {
    let usable = availability.is_ready();
    Button::new(id)
        .label(label)
        .when(selected, |this| this.primary())
        .when(!selected, |this| this.outline())
        .when(!usable, |this| this.opacity(0.6))
        .on_click(move |_, _, cx| {
            state::update_settings(cx, |settings| settings.backend.kind = kind);
        })
}

/// A dot in the status colour.
fn status_dot(status: &Status, cx: &App) -> impl IntoElement {
    let theme = cx.theme();
    let colour = match status {
        Status::Stopped => theme.muted_foreground,
        Status::Starting | Status::Stopping => theme.warning,
        Status::Running(_) => theme.success,
        Status::Failed(_) => theme.danger,
    };
    div().size(px(10.0)).rounded_full().bg(colour)
}

/// The status in words.
fn status_label(status: &Status, cx: &App) -> &'static str {
    let lang = state::lang(cx);
    match status {
        Status::Stopped => Text::StatusStopped.of(lang),
        Status::Starting => Text::StatusStarting.of(lang),
        Status::Running(_) => Text::StatusRunning.of(lang),
        Status::Stopping => Text::StatusStopping.of(lang),
        Status::Failed(_) => Text::StatusFailed.of(lang),
    }
}

/// The sentence under the status.
fn session_detail(status: &Status, name: &str, port: u16) -> String {
    match status {
        Status::Running(bound) => format!("{name}  {bound}"),
        Status::Failed(reason) => reason.clone(),
        _ => format!("{name}  {port}"),
    }
}

/// What the installation is doing, as a sentence.
fn progress_label(progress: &Progress, text: impl Fn(Text) -> &'static str) -> String {
    match progress {
        Progress::Preparing => text(Text::InstallPreparing).to_owned(),
        Progress::Downloading { received, total } => match total {
            Some(total) if *total > 0 => format!(
                "{}  {:.0}%  ({:.1} / {:.1} MB)",
                text(Text::InstallDownloading),
                *received as f64 / *total as f64 * 100.0,
                *received as f64 / 1_048_576.0,
                *total as f64 / 1_048_576.0
            ),
            _ => format!(
                "{}  {:.1} MB",
                text(Text::InstallDownloading),
                *received as f64 / 1_048_576.0
            ),
        },
        Progress::Extracting => text(Text::InstallExtracting).to_owned(),
        Progress::Verifying => text(Text::InstallVerifying).to_owned(),
        Progress::Done { version, .. } => format!("{}  {version}", text(Text::InstallDone)),
        Progress::Failed(reason) => format!("{}: {reason}", text(Text::InstallFailed)),
    }
}

/// The first characters of a device key, which is enough to tell two apart.
fn short_key(key: &str) -> String {
    key.chars().take(12).collect()
}
