// SPDX-License-Identifier: GPL-3.0-or-later
//! The settings pages, one page per group of choices.
//!
//! Every control here writes one field of [Settings](crate::settings::Settings) and saves
//! it. The receiver reads its configuration once when a session starts, so a page says
//! that a change takes effect next time, and there is nothing else to press.

use std::sync::Arc;

use gpui_kit::Axis;
use gpui_kit::component::ActiveTheme as _;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::progress::Progress as ProgressBar;
use gpui_kit::component::setting::{
    NumberFieldOptions, SettingField, SettingGroup, SettingItem, SettingPage, Settings,
};
use gpui_kit::component::{Sizable, Theme, ThemeMode, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::i18n::{Lang, Text};
use crate::render::detect::{self, Availability, MissingPart};
use crate::render::install::{self, Progress, Target};
use crate::settings::{
    AudioCodecChoice, BackendKind, DisplaySettings, LanguageChoice, LogLevel, PinChoice, PortsMode,
    Settings as Chosen, ThemeChoice, VideoCodecChoice,
};
use crate::state;
use crate::ui::RootView;
use crate::update::UpdateState;

/// The whole settings surface.
pub fn render(
    view: &mut RootView,
    _window: &mut Window,
    cx: &mut Context<RootView>,
) -> impl IntoElement {
    let install = view.installation().map(|(target, progress)| (*target, progress.clone()));
    let pages = build(cx, install);
    Settings::new("parlor-settings").sidebar_width(px(216.0)).pages(pages)
}

/// Builds every page.
fn build(cx: &App, install: Option<(Target, Progress)>) -> Vec<SettingPage> {
    vec![
        general(cx),
        network(cx),
        security(cx),
        video(cx),
        audio(cx),
        backend(cx, install),
        diagnostics(cx),
        about(cx),
    ]
}

/// The first page: what a device sees.
fn general(cx: &App) -> SettingPage {
    let lang = state::lang(cx);
    let text = move |label: Text| label.of(lang);
    let theme = match cx.global::<state::Parlor>().settings.appearance.theme {
        ThemeChoice::System => "system",
        ThemeChoice::Light => "light",
        ThemeChoice::Dark => "dark",
    };
    let language = match cx.global::<state::Parlor>().settings.appearance.language {
        LanguageChoice::System => "system",
        LanguageChoice::English => "en",
        LanguageChoice::Chinese => "zh",
    };
    SettingPage::new(text(Text::SettingsGeneral))
        .description(text(Text::SettingsDescription))
        .group(SettingGroup::new().title(text(Text::SessionTitle)).items(vec![
                SettingItem::new(
                    text(Text::DeviceId),
                    SettingField::input(
                        |cx| {
                            SharedString::from(
                                cx.global::<state::Parlor>().settings.receiver.device_id.clone(),
                            )
                        },
                        |value: SharedString, cx| {
                            state::update_settings(cx, |settings| {
                                settings.receiver.device_id = value.to_string();
                            });
                        },
                    ),
                )
                .layout(Axis::Vertical).description(text(Text::DeviceIdHint)),
                SettingItem::new(
                    text(Text::PaceVideo),
                    SettingField::switch(
                        |cx| cx.global::<state::Parlor>().settings.receiver.pace_video,
                        |value, cx| {
                            state::update_settings(cx, |settings| {
                                settings.receiver.pace_video = value;
                            });
                        },
                    ),
                )
                .layout(Axis::Vertical).description(text(Text::PaceVideoHint)),
                SettingItem::new(
                    text(Text::NoHold),
                    SettingField::switch(
                        |cx| cx.global::<state::Parlor>().settings.receiver.nohold,
                        |value, cx| {
                            state::update_settings(cx, |settings| {
                                settings.receiver.nohold = value;
                            });
                        },
                    ),
                )
                .layout(Axis::Vertical).description(text(Text::NoHoldHint)),
                SettingItem::new(
                    text(Text::FeedbackTimeout),
                    SettingField::number_input(
                        NumberFieldOptions { min: 0.0, max: 3600.0, step: 1.0 },
                        |cx| {
                            f64::from(
                                cx.global::<state::Parlor>()
                                    .settings
                                    .receiver
                                    .feedback_timeout
                                    .unwrap_or(0),
                            )
                        },
                        |value, cx| {
                            state::update_settings(cx, |settings| {
                                settings.receiver.feedback_timeout =
                                    (value >= 1.0).then_some(value.round() as u16);
                            });
                        },
                    ),
                )
                .layout(Axis::Vertical).description(text(Text::FeedbackTimeoutHint)),
                SettingItem::new(
                    text(Text::Theme),
                    SettingField::dropdown(
                        theme_options(lang),
                        move |_| SharedString::from(theme),
                        |value: SharedString, cx| choose_theme(&value, cx),
                    ),
                ),
                SettingItem::new(
                    text(Text::Language),
                    SettingField::dropdown(
                        vec![
                            ("system".into(), text(Text::LangSystem).into()),
                            ("en".into(), text(Text::LangEnglish).into()),
                            ("zh".into(), text(Text::LangChinese).into()),
                        ],
                        move |_| SharedString::from(language),
                        |value: SharedString, cx| {
                            let choice = match value.as_ref() {
                                "en" => LanguageChoice::English,
                                "zh" => LanguageChoice::Chinese,
                                _ => LanguageChoice::System,
                            };
                            state::update_settings(cx, |settings| {
                                settings.appearance.language = choice;
                            });
                        },
                    ),
                ),
            ]))
}

/// The theme choices, as the (value, label) pairs the field shows and answers with.
///
/// Shared with the window tests: they click the option and expect the writer to receive the
/// stored value, so a swap back to (label, value) fails a test instead of a user noticing.
pub(super) fn theme_options(lang: crate::i18n::Lang) -> Vec<(SharedString, SharedString)> {
    vec![
        ("system".into(), Text::ThemeSystem.of(lang).into()),
        ("light".into(), Text::ThemeLight.of(lang).into()),
        ("dark".into(), Text::ThemeDark.of(lang).into()),
    ]
}

/// The theme a stored value names; anything unrecognized follows the system.
fn theme_choice(value: &str) -> ThemeChoice {
    match value {
        "light" => ThemeChoice::Light,
        "dark" => ThemeChoice::Dark,
        _ => ThemeChoice::System,
    }
}

/// Applies a theme choice and remembers it.
fn choose_theme(value: &str, cx: &mut App) {
    let choice = theme_choice(value);
    state::update_settings(cx, |settings| settings.appearance.theme = choice);
    match choice {
        ThemeChoice::System => Theme::sync_system_appearance(None, cx),
        ThemeChoice::Light => Theme::change(ThemeMode::Light, None, cx),
        ThemeChoice::Dark => Theme::change(ThemeMode::Dark, None, cx),
    }
}

/// Where the receiver listens.
fn network(cx: &App) -> SettingPage {
    let lang = state::lang(cx);
    let text = move |label: Text| label.of(lang);
    let ports = match cx.global::<state::Parlor>().settings.network.ports {
        PortsMode::Automatic => "automatic",
        PortsMode::Legacy => "legacy",
        PortsMode::Custom => "custom",
    };
    let port_field = |field: fn(&mut Chosen) -> &mut u16, minimum: f64| {
        SettingField::number_input(
            NumberFieldOptions { min: minimum, max: 65535.0, step: 1.0 },
            move |cx| f64::from(*field(&mut cx.global::<state::Parlor>().settings.clone())),
            move |value, cx| {
                state::update_settings(cx, |settings| {
                    *field(settings) = value.round().clamp(1.0, 65535.0) as u16;
                });
            },
        )
    };
    let mut page = SettingPage::new(text(Text::SettingsNetwork)).group(
        SettingGroup::new().title(text(Text::SessionTitle)).items(vec![
            SettingItem::new(
                text(Text::ControlPort),
                port_field(|settings| &mut settings.network.port, 1.0),
            ),
            SettingItem::new(
                text(Text::BindAddress),
                SettingField::dropdown(
                    bind_options(&cx.global::<state::Parlor>().settings.network.bind, text),
                    |cx| {
                        SharedString::from(
                            cx.global::<state::Parlor>().settings.network.bind.clone(),
                        )
                    },
                    |value: SharedString, cx| {
                        state::update_settings(cx, |settings| {
                            settings.network.bind = value.to_string();
                        });
                    },
                ),
            ),
            SettingItem::new(
                text(Text::Advertise),
                SettingField::switch(
                    |cx| cx.global::<state::Parlor>().settings.network.advertise,
                    |value, cx| {
                        state::update_settings(cx, |settings| {
                            settings.network.advertise = value;
                        });
                    },
                ),
            )
            .layout(Axis::Vertical)
            .description(text(Text::AdvertiseHint)),
            SettingItem::new(
                text(Text::MediaPorts),
                SettingField::dropdown(
                    vec![
                        ("automatic".into(), text(Text::PortsAutomatic).into()),
                        ("legacy".into(), text(Text::PortsLegacy).into()),
                        ("custom".into(), text(Text::PortsCustom).into()),
                    ],
                    move |_| SharedString::from(ports),
                    |value: SharedString, cx| {
                        let mode = match value.as_ref() {
                            "legacy" => PortsMode::Legacy,
                            "custom" => PortsMode::Custom,
                            _ => PortsMode::Automatic,
                        };
                        state::update_settings(cx, |settings| settings.network.ports = mode);
                    },
                ),
            ),
        ]),
    );
    // Four port numbers that the receiver only binds when the mode says "custom"; under
    // any other mode they are values nothing reads.
    if ports == "custom" {
        page = page.group(SettingGroup::new().title(text(Text::MediaPorts)).items(vec![
            SettingItem::new(
                text(Text::TimingPort),
                port_field(|settings| &mut settings.network.timing, 1.0),
            ),
            SettingItem::new(
                text(Text::UdpControlPort),
                port_field(|settings| &mut settings.network.control, 1.0),
            ),
            SettingItem::new(
                text(Text::DataPort),
                port_field(|settings| &mut settings.network.data, 1.0),
            ),
            SettingItem::new(
                text(Text::MirrorPort),
                port_field(|settings| &mut settings.network.mirror, 1.0),
            ),
        ]));
    }
    page
}

/// Who may pair and who may start a session.
fn security(cx: &App) -> SettingPage {
    let lang = state::lang(cx);
    let text = move |label: Text| label.of(lang);
    let settings = &cx.global::<state::Parlor>().settings;
    let pin = match settings.auth.pin {
        PinChoice::OneTime => "one-time",
        PinChoice::Fixed => "fixed",
        PinChoice::Disabled => "disabled",
    };
    let restricted = settings.auth.restrict;
    let mut items = vec![SettingItem::new(
        text(Text::Pairing),
        SettingField::dropdown(
            vec![
                ("one-time".into(), text(Text::PinOneTime).into()),
                ("fixed".into(), text(Text::PinFixed).into()),
                ("disabled".into(), text(Text::PinDisabled).into()),
            ],
            move |_| SharedString::from(pin),
            |value: SharedString, cx| {
                let choice = match value.as_ref() {
                    "fixed" => PinChoice::Fixed,
                    "disabled" => PinChoice::Disabled,
                    _ => PinChoice::OneTime,
                };
                state::update_settings(cx, |settings| settings.auth.pin = choice);
            },
        ),
    )];
    if pin == "fixed" {
        items.push(SettingItem::new(
            text(Text::PinDigits),
            SettingField::number_input(
                NumberFieldOptions { min: 0.0, max: 9999.0, step: 1.0 },
                |cx| f64::from(cx.global::<state::Parlor>().settings.auth.pin_digits),
                |value, cx| {
                    state::update_settings(cx, |settings| {
                        settings.auth.pin_digits = value.round().clamp(0.0, 9999.0) as u16;
                    });
                },
            ),
        ));
    }
    items.push(
        SettingItem::new(
            text(Text::Password),
            SettingField::input(
                |cx| {
                    SharedString::from(cx.global::<state::Parlor>().settings.auth.password.clone())
                },
                |value: SharedString, cx| {
                    state::update_settings(cx, |settings| {
                        settings.auth.password = value.to_string();
                    });
                },
            ),
        )
        .layout(Axis::Vertical)
        .description(text(Text::PasswordHint)),
    );
    items.push(
        SettingItem::new(
            text(Text::RememberClients),
            SettingField::switch(
                |cx| cx.global::<state::Parlor>().settings.auth.remember_clients,
                |value, cx| {
                    state::update_settings(cx, |settings| settings.auth.remember_clients = value);
                },
            ),
        )
        .layout(Axis::Vertical)
        .description(text(Text::RememberClientsHint)),
    );
    items.push(
        SettingItem::new(
            text(Text::RestrictClients),
            SettingField::switch(
                |cx| cx.global::<state::Parlor>().settings.auth.restrict,
                |value, cx| {
                    state::update_settings(cx, |settings| settings.auth.restrict = value);
                },
            ),
        )
        .layout(Axis::Vertical)
        .description(text(Text::RestrictClientsHint)),
    );
    items.push(SettingItem::new(
        text(Text::BlockedDevices),
        SettingField::input(
            |cx| SharedString::from(join_list(&cx.global::<state::Parlor>().settings.auth.block)),
            |value: SharedString, cx| {
                state::update_settings(cx, |settings| settings.auth.block = split_list(&value));
            },
        ),
    ));
    if restricted {
        items.push(
            SettingItem::new(
                text(Text::AllowedDevices),
                SettingField::input(
                    |cx| {
                        SharedString::from(join_list(
                            &cx.global::<state::Parlor>().settings.auth.allow,
                        ))
                    },
                    |value: SharedString, cx| {
                        state::update_settings(cx, |settings| {
                            settings.auth.allow = split_list(&value)
                        });
                    },
                ),
            )
            .layout(Axis::Vertical)
            .description(text(Text::OnePerLine)),
        );
    }
    items.push(known_devices(lang));
    SettingPage::new(text(Text::SettingsSecurity))
        .group(SettingGroup::new().title(text(Text::Pairing)).items(items))
}

/// The list of remembered devices, with the two destructive actions.
fn known_devices(lang: Lang) -> SettingItem {
    SettingItem::render(move |_, _window, cx| {
        let text = |label: Text| label.of(state::lang(cx));
        let keys: Vec<String> = cx
            .global::<state::Parlor>()
            .identity
            .clients()
            .iter()
            .map(|key| crate::settings::identity::hex(key))
            .collect();
        v_flex()
            .gap_2()
            .items_end()
            .when(keys.is_empty(), |this| this.child(div().text_xs().child(text(Text::NoDevices))))
            .children(keys.into_iter().map(|key| {
                let id = format!("forget-{key}");
                let parsed = crate::settings::identity::parse_hex32(&key).ok();
                h_flex().gap_2().items_center().child(div().text_xs().child(key.clone())).child(
                    Button::new(id).xsmall().label(text(Text::Forget)).on_click(move |_, _, cx| {
                        if let Some(key) = parsed {
                            state::update_identity(cx, |identity| {
                                identity.forget(&key);
                            });
                        }
                    }),
                )
            }))
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        Button::new("forget-all")
                            .xsmall()
                            .danger()
                            .label(text(Text::ForgetAll))
                            .on_click(|_, _, cx| {
                                state::update_identity(cx, |identity| identity.forget_all());
                            }),
                    )
                    .child(
                        Button::new("regenerate")
                            .xsmall()
                            .outline()
                            .label(text(Text::RegenerateIdentity))
                            .on_click(move |_, _, cx| {
                                let paths = cx.global::<state::Parlor>().paths.clone();
                                state::update_identity(cx, move |identity| {
                                    let _ = identity.regenerate(&paths);
                                });
                                let _ = lang;
                            }),
                    ),
            )
    })
}

/// The picture a device is asked for.
fn video(cx: &App) -> SettingPage {
    let lang = state::lang(cx);
    let text = move |label: Text| label.of(lang);
    SettingPage::new(text(Text::SettingsVideo)).group(
        SettingGroup::new().title(text(Text::QualityTitle)).items(vec![
            SettingItem::new(
                text(Text::Hevc),
                SettingField::switch(
                    |cx| cx.global::<state::Parlor>().settings.video.hevc,
                    |value, cx| state::update_settings(cx, |settings| settings.video.hevc = value),
                ),
            )
            .layout(Axis::Vertical)
            .description(text(Text::HevcHint)),
            SettingItem::new(
                text(Text::Overscanned),
                SettingField::switch(
                    |cx| cx.global::<state::Parlor>().settings.display.overscanned,
                    |value, cx| {
                        state::update_settings(cx, |settings| settings.display.overscanned = value)
                    },
                ),
            )
            .layout(Axis::Vertical)
            .description(text(Text::OverscannedHint)),
            SettingItem::new(
                text(Text::Resolution),
                SettingField::dropdown(
                    resolution_options(
                        cx.global::<state::Parlor>().settings.display.width,
                        cx.global::<state::Parlor>().settings.display.height,
                    ),
                    |cx| {
                        let display = &cx.global::<state::Parlor>().settings.display;
                        SharedString::from(format!("{}x{}", display.width, display.height))
                    },
                    |value: SharedString, cx| {
                        let Some((width, height)) = parse_resolution(&value) else {
                            return;
                        };
                        state::update_settings(cx, |settings| {
                            settings.display.width = width;
                            settings.display.height = height;
                        });
                    },
                ),
            )
            .layout(Axis::Vertical)
            .description(text(Text::ResolutionHint)),
            SettingItem::new(
                text(Text::FrameRate),
                SettingField::dropdown(
                    DisplaySettings::FRAME_RATES
                        .iter()
                        .map(|fps| {
                            (
                                SharedString::from(fps.to_string()),
                                SharedString::from(fps.to_string()),
                            )
                        })
                        .collect(),
                    |cx| {
                        SharedString::from(
                            cx.global::<state::Parlor>().settings.display.max_fps.to_string(),
                        )
                    },
                    |value: SharedString, cx| {
                        let fps = value.parse::<u32>().unwrap_or(30).clamp(1, 240);
                        state::update_settings(cx, |settings| settings.display.max_fps = fps);
                    },
                ),
            )
            .layout(Axis::Vertical)
            .description(text(Text::FrameRateHint)),
            SettingItem::new(
                text(Text::RefreshRate),
                SettingField::dropdown(
                    refresh_options(cx.global::<state::Parlor>().settings.display.refresh_rate),
                    |cx| {
                        SharedString::from(format!(
                            "{:.0}",
                            cx.global::<state::Parlor>().settings.display.refresh_rate
                        ))
                    },
                    |value: SharedString, cx| {
                        let Ok(hz) = value.parse::<f64>() else {
                            return;
                        };
                        state::update_settings(cx, |settings| {
                            settings.display.refresh_rate = hz.clamp(1.0, 240.0);
                        });
                    },
                ),
            ),
        ]),
    )
}

/// The audio a session expects.
fn audio(cx: &App) -> SettingPage {
    let lang = state::lang(cx);
    let text = move |label: Text| label.of(lang);
    let codec = match cx.global::<state::Parlor>().settings.audio.codec {
        AudioCodecChoice::Alac => "alac",
        AudioCodecChoice::AacEld => "aac-eld",
    };
    SettingPage::new(text(Text::SettingsAudio)).group(
        SettingGroup::new().title(text(Text::AudioCodec)).items(vec![
            SettingItem::new(
                text(Text::AudioCodec),
                SettingField::dropdown(
                    vec![
                        ("alac".into(), text(Text::CodecAlac).into()),
                        ("aac-eld".into(), text(Text::CodecAacEld).into()),
                    ],
                    move |_| SharedString::from(codec),
                    |value: SharedString, cx| {
                        let choice = if value.as_ref() == "alac" {
                            AudioCodecChoice::Alac
                        } else {
                            AudioCodecChoice::AacEld
                        };
                        state::update_settings(cx, |settings| settings.audio.codec = choice);
                    },
                ),
            ),
            SettingItem::new(
                text(Text::Latency),
                SettingField::number_input(
                    NumberFieldOptions {
                        min: 0.0,
                        max: f64::from(crate::settings::AudioSettings::MAX_LATENCY_MS),
                        step: 10.0,
                    },
                    |cx| f64::from(cx.global::<state::Parlor>().settings.audio.latency_ms),
                    |value, cx| {
                        state::update_settings(cx, |settings| {
                            settings.audio.latency_ms = value.round().clamp(
                                0.0,
                                f64::from(crate::settings::AudioSettings::MAX_LATENCY_MS),
                            ) as u32;
                        });
                    },
                ),
            )
            .layout(Axis::Vertical)
            .description(text(Text::LatencyHint)),
            SettingItem::new(
                text(Text::InitialVolume),
                SettingField::number_input(
                    NumberFieldOptions { min: 0.0, max: 100.0, step: 1.0 },
                    |cx| f64::from(cx.global::<state::Parlor>().settings.audio.initial_volume),
                    |value, cx| {
                        state::update_settings(cx, |settings| {
                            settings.audio.initial_volume = value.round().clamp(0.0, 100.0) as u8;
                        });
                    },
                ),
            ),
            SettingItem::new(
                text(Text::Taper),
                SettingField::switch(
                    |cx| cx.global::<state::Parlor>().settings.audio.taper,
                    |value, cx| state::update_settings(cx, |settings| settings.audio.taper = value),
                ),
            )
            .layout(Axis::Vertical)
            .description(text(Text::TaperHint)),
            SettingItem::new(
                text(Text::LowDb),
                SettingField::number_input(
                    NumberFieldOptions { min: -90.0, max: 0.0, step: 1.0 },
                    |cx| cx.global::<state::Parlor>().settings.audio.low_db,
                    |value, cx| {
                        state::update_settings(cx, |settings| {
                            settings.audio.low_db = value.clamp(-90.0, 0.0);
                        });
                    },
                ),
            ),
            SettingItem::new(
                text(Text::HighDb),
                SettingField::number_input(
                    NumberFieldOptions { min: -90.0, max: 0.0, step: 1.0 },
                    |cx| cx.global::<state::Parlor>().settings.audio.high_db,
                    |value, cx| {
                        state::update_settings(cx, |settings| {
                            settings.audio.high_db = value.clamp(-90.0, 0.0);
                        });
                    },
                ),
            ),
        ]),
    )
}

/// Which backend plays, and every backend the machine can play through.
fn backend(cx: &App, install: Option<(Target, Progress)>) -> SettingPage {
    let lang = state::lang(cx);
    let text = move |label: Text| label.of(lang);
    let parlor = state::parlor(cx);
    let backends = parlor.backends.clone();
    let paths = parlor.paths.clone();
    let kind = parlor.settings.backend.kind;
    let progress = |target: Target| {
        install
            .as_ref()
            .filter(|(installing, _)| *installing == target)
            .map(|(_, progress)| progress.clone())
    };

    let mut items = vec![SettingItem::new(
        text(Text::BackendKind),
        SettingField::dropdown(
            vec![
                ("gstreamer".into(), text(Text::GStreamer).into()),
                ("ffmpeg".into(), text(Text::Ffmpeg).into()),
            ],
            move |_| {
                SharedString::from(match kind {
                    BackendKind::GStreamer => "gstreamer",
                    BackendKind::Ffmpeg => "ffmpeg",
                })
            },
            |value: SharedString, cx| {
                let kind = if value.as_ref() == "ffmpeg" {
                    BackendKind::Ffmpeg
                } else {
                    BackendKind::GStreamer
                };
                state::update_settings(cx, |settings| settings.backend.kind = kind);
            },
        ),
    )];
    items.push(availability_item(
        target_name(Text::GStreamer, lang),
        target_name(Text::GStreamer, lang),
        Target::GStreamer,
        backends.gstreamer.clone(),
        paths.clone(),
        progress(Target::GStreamer),
    ));
    items.extend(gstreamer_items(cx));
    items.push(availability_item(
        target_name(Text::Ffmpeg, lang),
        target_name(Text::Ffmpeg, lang),
        Target::Ffmpeg,
        backends.ffmpeg.clone(),
        paths.clone(),
        progress(Target::Ffmpeg),
    ));
    items.extend(ffmpeg_items(cx));
    let folder = std::env::current_exe()
        .ok()
        .and_then(|executable| executable.parent().map(std::path::Path::to_path_buf));
    if let Some(folder) = folder {
        items.push(SettingItem::render(move |_, _, _| {
            Button::new("open-application-folder").xsmall().outline().label("Parlor").on_click({
                let folder = folder.clone();
                move |_, _, _| {
                    let _ = std::process::Command::new("explorer").arg(&folder).spawn();
                }
            })
        }));
    }
    SettingPage::new(text(Text::SettingsBackend))
        .group(SettingGroup::new().title(text(Text::BackendKind)).items(items))
}

fn target_name(label: Text, lang: Lang) -> &'static str {
    label.of(lang)
}

/// One backend: what it is, whether it is here, and what it is doing.
fn availability_item(
    _name: &'static str,
    _label: &'static str,
    target: Target,
    availability: Availability,
    paths: crate::settings::store::Paths,
    progress: Option<Progress>,
) -> SettingItem {
    SettingItem::render(move |_, _window, cx| {
        let lang = state::lang(cx);
        let text = move |label: Text| label.of(lang);
        let (summary, colour) = match &availability {
            Availability::Ready { version, .. } => {
                (format!("{} · {version}", text(Text::Available)), cx.theme().success)
            }
            Availability::Missing { part, .. } => (
                match part {
                    MissingPart::Plugin => text(Text::PluginMissing).to_owned(),
                    MissingPart::Runtime => text(Text::NotInstalled).to_owned(),
                },
                cx.theme().muted_foreground,
            ),
        };
        let name = match target {
            Target::Ffmpeg => text(Text::Ffmpeg),
            Target::GStreamer => text(Text::GStreamer),
        };
        let present = install::installed(target, &paths);
        let installable =
            !matches!(availability, Availability::Missing { part: MissingPart::Plugin, .. });
        let sender = cx.global::<state::Parlor>().installs.clone();
        let remove_paths = paths.clone();
        let installing = progress.is_some();
        let row = h_flex()
            .justify_between()
            .items_center()
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(div().text_sm().font_weight(FontWeight::SEMIBOLD).child(name))
                    .child(div().text_sm().text_color(colour).child(summary)),
            )
            .child(
                h_flex()
                    .gap_2()
                    .when(present, |this| {
                        this.child(
                            Button::new(SharedString::from(format!(
                                "reinstall-{}",
                                target.directory()
                            )))
                            .xsmall()
                            .outline()
                            .label(text(Text::Reinstall))
                            .on_click({
                                let sender = sender.clone();
                                move |_, _, cx| {
                                    let _ = sender.try_send(target);
                                    cx.refresh_windows();
                                }
                            }),
                        )
                        .child(
                            Button::new(SharedString::from(format!(
                                "remove-{}",
                                target.directory()
                            )))
                            .xsmall()
                            .label(text(Text::Uninstall))
                            .on_click({
                                let paths = remove_paths.clone();
                                move |_, _, cx| {
                                    let _ = install::remove(target, &paths);
                                    state::reprobe(cx);
                                }
                            }),
                        )
                    })
                    .when(!present && installable && !installing, |this| {
                        this.child(
                            Button::new(SharedString::from(format!(
                                "install-{}",
                                target.directory()
                            )))
                            .xsmall()
                            .primary()
                            .label(text(Text::InstallNow))
                            .on_click({
                                let sender = sender.clone();
                                move |_, _, cx| {
                                    let _ = sender.try_send(target);
                                    cx.refresh_windows();
                                }
                            }),
                        )
                    }),
            );
        let bar = progress.clone().map(|progress| {
            v_flex()
                .gap_1()
                .child(
                    ProgressBar::new(SharedString::from(format!("install-{}", target.directory())))
                        .value(progress.fraction() as f32 * 100.0),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(install_phase(&progress, text)),
                )
        });
        v_flex().gap_2().w_full().child(row).when_some(bar, |this, bar| this.child(bar))
    })
}

/// What an installation is doing, as one short line.
fn install_phase(progress: &Progress, text: impl Fn(Text) -> &'static str) -> String {
    match progress {
        Progress::Preparing => text(Text::InstallPreparing).to_owned(),
        Progress::Downloading { received, total } => match total {
            Some(total) if *total > 0 => format!(
                "{}  {:.0}%",
                text(Text::InstallDownloading),
                *received as f64 / *total as f64 * 100.0
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

/// The FFmpeg backend's own choices.
///
/// The coding a warm sink starts for is shown only while a warm sink is started: with
/// nothing kept alive there is no coding to choose.
fn ffmpeg_items(cx: &App) -> Vec<SettingItem> {
    let lang = state::lang(cx);
    let text = move |label: Text| label.of(lang);
    let chosen = &cx.global::<state::Parlor>().settings.backend.ffmpeg;
    let warm_video = chosen.warm_video;
    let warm = match chosen.warm_codec {
        VideoCodecChoice::H264 => "h264",
        VideoCodecChoice::H265 => "h265",
    };
    let mut items = vec![
        SettingItem::new(
            text(Text::Decoder),
            SettingField::dropdown(
                decoder_options(cx),
                |cx| {
                    SharedString::from(
                        cx.global::<state::Parlor>().settings.backend.ffmpeg.decoder.clone(),
                    )
                },
                |value: SharedString, cx| {
                    state::update_settings(cx, |settings| {
                        settings.backend.ffmpeg.decoder = value.to_string();
                    });
                },
            ),
        )
        .layout(Axis::Vertical),
        SettingItem::new(
            text(Text::AudioSink),
            SettingField::dropdown(
                value_options(
                    &["ffplay -nodisp".to_owned(), "ffplay -nodisp -volume 0".to_owned()],
                    &cx.global::<state::Parlor>().settings.backend.ffmpeg.audio_sink,
                    text(Text::AutoChoice).into(),
                ),
                |cx| {
                    SharedString::from(
                        cx.global::<state::Parlor>().settings.backend.ffmpeg.audio_sink.clone(),
                    )
                },
                |value: SharedString, cx| {
                    state::update_settings(cx, |settings| {
                        settings.backend.ffmpeg.audio_sink = value.to_string();
                    });
                },
            ),
        ),
        SettingItem::new(
            text(Text::VideoSink),
            SettingField::dropdown(
                value_options(
                    &[
                        "ffplay -framedrop -x 1280 -y 720".to_owned(),
                        "ffplay -framedrop -x 1600 -y 900".to_owned(),
                        "ffplay -framedrop -fs".to_owned(),
                    ],
                    &cx.global::<state::Parlor>().settings.backend.ffmpeg.video_sink,
                    text(Text::AutoChoice).into(),
                ),
                |cx| {
                    SharedString::from(
                        cx.global::<state::Parlor>().settings.backend.ffmpeg.video_sink.clone(),
                    )
                },
                |value: SharedString, cx| {
                    state::update_settings(cx, |settings| {
                        settings.backend.ffmpeg.video_sink = value.to_string();
                    });
                },
            ),
        ),
        SettingItem::new(
            text(Text::WarmVideo),
            SettingField::switch(
                |cx| cx.global::<state::Parlor>().settings.backend.ffmpeg.warm_video,
                |value, cx| {
                    state::update_settings(cx, |settings| {
                        settings.backend.ffmpeg.warm_video = value
                    });
                },
            ),
        )
        .layout(Axis::Vertical),
    ];
    if warm_video {
        items.push(
            SettingItem::new(
                text(Text::WarmCodec),
                SettingField::dropdown(
                    vec![
                        ("h264".into(), text(Text::H264Chain).into()),
                        ("h265".into(), text(Text::H265Chain).into()),
                    ],
                    move |_| SharedString::from(warm),
                    |value: SharedString, cx| {
                        let choice = if value.as_ref() == "h265" {
                            VideoCodecChoice::H265
                        } else {
                            VideoCodecChoice::H264
                        };
                        state::update_settings(cx, |settings| {
                            settings.backend.ffmpeg.warm_codec = choice;
                        });
                    },
                ),
            )
            .layout(Axis::Vertical),
        );
    }
    items.push(
        SettingItem::new(
            text(Text::HardwareDecoding),
            SettingField::switch(
                |cx| cx.global::<state::Parlor>().settings.backend.ffmpeg.hardware,
                |value, cx| {
                    state::update_settings(cx, |settings| {
                        settings.backend.ffmpeg.hardware = value;
                    });
                },
            ),
        )
        .layout(Axis::Vertical),
    );
    items
}

/// The GStreamer backend's own choices.
fn gstreamer_items(cx: &App) -> Vec<SettingItem> {
    let lang = state::lang(cx);
    let text = move |label: Text| label.of(lang);
    let elements = cx.global::<state::Parlor>().gst_elements.clone();
    let element_item = |label: &'static str,
                        get: fn(&Chosen) -> String,
                        set: fn(&mut Chosen) -> &mut String,
                        candidates: &'static [&'static str]| {
        let current = get(&cx.global::<state::Parlor>().settings);
        SettingItem::new(
            label,
            SettingField::dropdown(
                element_options(&elements, candidates, &current, text(Text::AutoChoice).into()),
                move |cx| SharedString::from(get(&cx.global::<state::Parlor>().settings)),
                move |value: SharedString, cx| {
                    state::update_settings(cx, |settings| {
                        *set(settings) = value.to_string();
                    });
                },
            ),
        )
        .layout(Axis::Vertical)
    };
    vec![
        SettingItem::new(
            text(Text::Location),
            SettingField::dropdown(
                launch_options(cx),
                |cx| {
                    SharedString::from(
                        cx.global::<state::Parlor>().settings.backend.gstreamer.launch_path.clone(),
                    )
                },
                |value: SharedString, cx| {
                    state::update_settings(cx, |settings| {
                        settings.backend.gstreamer.launch_path = value.to_string();
                    });
                },
            ),
        )
        .layout(Axis::Vertical),
        element_item(
            text(Text::AudioSink),
            |settings| settings.backend.gstreamer.audio_sink.clone(),
            |settings| &mut settings.backend.gstreamer.audio_sink,
            &["wasapisink", "autoaudiosink", "directsoundsink", "fakesink"],
        ),
        element_item(
            text(Text::VideoSink),
            |settings| settings.backend.gstreamer.video_sink.clone(),
            |settings| &mut settings.backend.gstreamer.video_sink,
            &["d3d11videosink", "autovideosink", "glimagesink", "fakesink"],
        ),
        element_item(
            text(Text::H264Parser),
            |settings| settings.backend.gstreamer.h264_parser.clone(),
            |settings| &mut settings.backend.gstreamer.h264_parser,
            &["h264parse"],
        ),
        element_item(
            text(Text::H264Decoder),
            |settings| settings.backend.gstreamer.h264_decoder.clone(),
            |settings| &mut settings.backend.gstreamer.h264_decoder,
            &["d3d11h264dec", "avdec_h264", "openh264dec"],
        ),
        element_item(
            text(Text::H265Parser),
            |settings| settings.backend.gstreamer.h265_parser.clone(),
            |settings| &mut settings.backend.gstreamer.h265_parser,
            &["h265parse"],
        ),
        element_item(
            text(Text::H265Decoder),
            |settings| settings.backend.gstreamer.h265_decoder.clone(),
            |settings| &mut settings.backend.gstreamer.h265_decoder,
            &["d3d11h265dec", "avdec_h265"],
        ),
        SettingItem::new(
            text(Text::WarmVideo),
            SettingField::switch(
                |cx| cx.global::<state::Parlor>().settings.backend.gstreamer.warm_video,
                |value, cx| {
                    state::update_settings(cx, |settings| {
                        settings.backend.gstreamer.warm_video = value
                    });
                },
            ),
        )
        .layout(Axis::Vertical),
    ]
}

/// What is recorded for a diagnosis.
fn diagnostics(cx: &App) -> SettingPage {
    let lang = state::lang(cx);
    let text = move |label: Text| label.of(lang);
    let level = match cx.global::<state::Parlor>().settings.diagnostics.log_level {
        LogLevel::Off => "off",
        LogLevel::Error => "error",
        LogLevel::Warn => "warn",
        LogLevel::Info => "info",
        LogLevel::Debug => "debug",
        LogLevel::Trace => "trace",
    };
    SettingPage::new(text(Text::SettingsDiagnostics)).group(
        SettingGroup::new().title(text(Text::Log)).items(vec![
            SettingItem::new(
                text(Text::LogLevel),
                SettingField::dropdown(
                    vec![
                        ("off".into(), text(Text::LevelOff).into()),
                        ("error".into(), text(Text::LevelError).into()),
                        ("warn".into(), text(Text::LevelWarn).into()),
                        ("info".into(), text(Text::LevelInfo).into()),
                        ("debug".into(), text(Text::LevelDebug).into()),
                        ("trace".into(), text(Text::LevelTrace).into()),
                    ],
                    move |_| SharedString::from(level),
                    |value: SharedString, cx| {
                        let level = match value.as_ref() {
                            "off" => LogLevel::Off,
                            "error" => LogLevel::Error,
                            "warn" => LogLevel::Warn,
                            "debug" => LogLevel::Debug,
                            "trace" => LogLevel::Trace,
                            _ => LogLevel::Info,
                        };
                        state::update_settings(cx, |settings| {
                            settings.diagnostics.log_level = level
                        });
                        cx.global::<state::Parlor>().log.set_level(level.filter());
                    },
                ),
            ),
            switch_item(
                text(Text::TraceLog),
                text(Text::TraceLogHint),
                |cx| cx.global::<state::Parlor>().settings.diagnostics.trace_log,
                |value, cx| {
                    state::update_settings(cx, |settings| settings.diagnostics.trace_log = value);
                },
            ),
            switch_item(
                text(Text::DumpVideo),
                text(Text::DumpVideoHint),
                |cx| cx.global::<state::Parlor>().settings.diagnostics.dump_video,
                |value, cx| {
                    state::update_settings(cx, |settings| settings.diagnostics.dump_video = value);
                },
            ),
            switch_item(
                text(Text::DumpAudio),
                text(Text::DumpAudioHint),
                |cx| cx.global::<state::Parlor>().settings.diagnostics.dump_audio,
                |value, cx| {
                    state::update_settings(cx, |settings| settings.diagnostics.dump_audio = value);
                },
            ),
            switch_item(
                text(Text::StreamingReports),
                text(Text::StreamingReportsHint),
                |cx| cx.global::<state::Parlor>().settings.diagnostics.streaming_reports,
                |value, cx| {
                    state::update_settings(cx, |settings| {
                        settings.diagnostics.streaming_reports = value;
                    });
                },
            ),
            SettingItem::render(move |_, _window, cx| {
                let text = |label: Text| label.of(state::lang(cx));
                let logs = cx.global::<state::Parlor>().paths.logs().display().to_string();
                h_flex().gap_2().items_center().child(
                    Button::new("open-logs").xsmall().label(text(Text::OpenLogs)).on_click(
                        move |_, _, _| {
                            let _ = std::process::Command::new("explorer").arg(&logs).spawn();
                        },
                    ),
                )
            }),
            SettingItem::render(move |_, _window, cx| {
                let text = |label: Text| label.of(state::lang(cx));
                let lines = cx.global::<state::Parlor>().log.lines();
                let tail: Vec<String> =
                    lines.iter().rev().take(30).rev().map(|line| line.to_string()).collect();
                v_flex()
                    .gap_1()
                    .when(tail.is_empty(), |this| {
                        this.child(div().text_xs().child(text(Text::NoLog)))
                    })
                    .children(
                        tail.into_iter()
                            .map(|line| div().text_xs().font_family("Consolas").child(line)),
                    )
            }),
        ]),
    )
}

/// A switch with a description.
fn switch_item(
    title: &'static str,
    description: &'static str,
    get: fn(&App) -> bool,
    set: fn(bool, &mut App),
) -> SettingItem {
    SettingItem::new(title, SettingField::switch(get, set))
        .layout(Axis::Vertical)
        .description(description)
}

/// Where the version, the licence and the project are.
fn about(cx: &App) -> SettingPage {
    let lang = state::lang(cx);
    let text = move |label: Text| label.of(lang);
    SettingPage::new(text(Text::SettingsAbout)).group(
        SettingGroup::new().title(text(Text::AboutTitle)).items(vec![
            SettingItem::render(move |_, _window, cx| {
                let text = |label: Text| label.of(state::lang(cx));
                let icon = img(Arc::new(Image::from_bytes(
                    ImageFormat::Png,
                    include_bytes!("../../../../assets/icon-256.png").to_vec(),
                )))
                .size(px(96.0));
                v_flex()
                    .w_full()
                    .items_center()
                    .gap_2()
                    .child(icon)
                    .child(div().text_xl().font_weight(FontWeight::BOLD).child("Parlor"))
                    .child(div().text_sm().text_color(cx.theme().muted_foreground).child(format!(
                        "{} {}",
                        text(Text::Version),
                        env!("CARGO_PKG_VERSION")
                    )))
            }),
            SettingItem::render(move |_, _window, cx| {
                let text = |label: Text| label.of(state::lang(cx));
                let update = cx.global::<state::Parlor>().update.clone();
                let (message, label) = match &update {
                    UpdateState::Idle => (String::new(), text(Text::CheckUpdates).to_owned()),
                    UpdateState::Checking => {
                        (text(Text::UpdateChecking).to_owned(), text(Text::CheckUpdates).to_owned())
                    }
                    UpdateState::UpToDate => {
                        (text(Text::UpdateUpToDate).to_owned(), text(Text::CheckUpdates).to_owned())
                    }
                    UpdateState::Available(version) => (
                        format!("{} {version}", text(Text::UpdateAvailable)),
                        text(Text::Download).to_owned(),
                    ),
                    UpdateState::Failed(reason) => (
                        format!("{}: {reason}", text(Text::UpdateFailed)),
                        text(Text::CheckUpdates).to_owned(),
                    ),
                };
                let downloads = matches!(update, UpdateState::Available(_));
                v_flex()
                    .w_full()
                    .items_center()
                    .gap_3()
                    .when(!message.is_empty(), |this| {
                        this.child(
                            div().text_sm().text_color(cx.theme().muted_foreground).child(message),
                        )
                    })
                    .child(
                        h_flex()
                            .gap_2()
                            .justify_center()
                            .child(
                                Button::new("check-updates")
                                    .large()
                                    .primary()
                                    .label(label)
                                    .on_click(move |_, _, cx| {
                                        if downloads {
                                            cx.open_url(crate::update::RELEASES_URL);
                                        } else {
                                            state::request_update_check(cx);
                                        }
                                    }),
                            )
                            .child(
                                Button::new("homepage")
                                    .large()
                                    .outline()
                                    .label(text(Text::Homepage))
                                    .on_click(|_, _, cx| {
                                        cx.open_url(
                                            "https://github.com/rust-airplay/parlor-desktop",
                                        );
                                    }),
                            ),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child("GPL-3.0-or-later"),
                    )
            }),
        ]),
    )
}

/// Splits a comma-separated list.
fn split_list(value: &str) -> Vec<String> {
    value.split(',').map(str::trim).filter(|entry| !entry.is_empty()).map(str::to_owned).collect()
}

/// Writes a list back as comma-separated text.
fn join_list(values: &[String]) -> String {
    values.join(", ")
}

/// The addresses this machine can listen on, with the one the settings hold.
fn bind_options(
    current: &str,
    text: impl Fn(Text) -> &'static str + Copy,
) -> Vec<(SharedString, SharedString)> {
    let mut options = vec![
        ("0.0.0.0".into(), text(Text::BindAllInterfaces).into()),
        ("127.0.0.1".into(), text(Text::BindThisMachine).into()),
    ];
    if let Some(lan) = local_address() {
        let label = format!("{lan} · {}", text(Text::BindLocalNetwork));
        options.push((SharedString::from(&lan), label.into()));
    }
    add_current(&mut options, current);
    options
}

/// The address of this machine on its local network, learned without sending anything.
fn local_address() -> Option<String> {
    let socket = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    Some(socket.local_addr().ok()?.ip().to_string())
}

/// One option per preset size, plus whatever size the settings hold.
fn resolution_options(width: u32, height: u32) -> Vec<(SharedString, SharedString)> {
    let mut options: Vec<(SharedString, SharedString)> = crate::settings::PRESETS
        .iter()
        .map(|(width, height)| {
            let label = format!("{width}x{height}");
            (label.clone().into(), label.into())
        })
        .collect();
    add_current(&mut options, &format!("{width}x{height}"));
    options
}

/// The size a choice names.
fn parse_resolution(value: &str) -> Option<(u32, u32)> {
    let (width, height) = value.split_once('x')?;
    Some((width.trim().parse().ok()?, height.trim().parse().ok()?))
}

/// The refresh rates a display choice offers, plus the one the settings hold.
fn refresh_options(current: f64) -> Vec<(SharedString, SharedString)> {
    let mut rates = vec![30, 50, 60, 90, 120, 144, 240];
    let rounded = current.round() as u32;
    if !rates.contains(&rounded) {
        rates.push(rounded);
    }
    rates
        .into_iter()
        .map(|hz| {
            let label = format!("{hz}");
            (label.clone().into(), label.into())
        })
        .collect()
}

/// Options for a GStreamer element: the automatic choice, the candidates this runtime has,
/// and a value the settings already hold.
fn element_options(
    elements: &std::collections::BTreeSet<String>,
    candidates: &[&str],
    current: &str,
    auto: SharedString,
) -> Vec<(SharedString, SharedString)> {
    let mut options = vec![(SharedString::from(""), auto)];
    for name in candidates {
        if elements.contains(*name) {
            options.push((SharedString::from(*name), SharedString::from(*name)));
        }
    }
    add_current(&mut options, current);
    options
}

/// Options from a named list, plus a value the settings already hold.
fn value_options(
    values: &[String],
    current: &str,
    auto: SharedString,
) -> Vec<(SharedString, SharedString)> {
    let mut options = vec![(SharedString::from(""), auto)];
    for value in values {
        options.push((SharedString::from(value), SharedString::from(value)));
    }
    add_current(&mut options, current);
    options
}

/// Adds the stored value as an option when the list does not have it, so a value written by
/// hand stays visible and selectable.
fn add_current(options: &mut Vec<(SharedString, SharedString)>, current: &str) {
    if !current.is_empty() && !options.iter().any(|(value, _)| value.as_ref() == current) {
        options.push((SharedString::from(current), SharedString::from(current)));
    }
}

/// The launchers this machine can offer.
fn launch_options(cx: &App) -> Vec<(SharedString, SharedString)> {
    let parlor = state::parlor(cx);
    let mut values = Vec::new();
    if let Some(found) = detect::gst_launch(&parlor.settings) {
        values.push(found.display().to_string());
    }
    let installed = parlor.paths.backend("gstreamer").join("bin").join("gst-launch-1.0.exe");
    if installed.is_file() {
        values.push(installed.display().to_string());
    }
    let auto = SharedString::from(Text::AutoChoice.of(state::lang(cx)));
    value_options(&values, &parlor.settings.backend.gstreamer.launch_path, auto)
}

/// The FFmpeg programs this machine can offer as the decoder.
fn decoder_options(cx: &App) -> Vec<(SharedString, SharedString)> {
    let parlor = state::parlor(cx);
    let mut values = Vec::new();
    if let Some(programs) = detect::ffmpeg_programs(&parlor.settings) {
        values.push(programs.decoder.display().to_string());
    }
    let installed = parlor.paths.backend("ffmpeg").join("bin").join("ffmpeg.exe");
    if installed.is_file() {
        values.push(installed.display().to_string());
    }
    let auto = SharedString::from(Text::AutoChoice.of(state::lang(cx)));
    value_options(&values, &parlor.settings.backend.ffmpeg.decoder, auto)
}

#[cfg(test)]
mod tests {
    use super::{join_list, split_list};

    #[test]
    fn the_element_choices_are_what_the_runtime_has_plus_the_stored_value() {
        let elements = ["wasapisink".to_owned(), "fakesink".to_owned()].into_iter().collect();
        let options = super::element_options(
            &elements,
            &["wasapisink", "autoaudiosink", "fakesink"],
            "directsoundsink",
            "auto".into(),
        );
        let values: Vec<&str> = options.iter().map(|(value, _)| value.as_ref()).collect();
        assert_eq!(values, ["", "wasapisink", "fakesink", "directsoundsink"]);
    }

    #[test]
    fn a_resolution_choice_names_both_sides() {
        let options = super::resolution_options(1280, 720);
        assert!(options.iter().any(|(value, _)| value == "1280x720"));
        let stored = super::resolution_options(1000, 600);
        assert!(stored.iter().any(|(value, _)| value == "1000x600"));
        assert_eq!(super::parse_resolution("1920x1080"), Some((1920, 1080)));
        assert_eq!(super::parse_resolution("1920"), None);
    }

    #[test]
    fn a_typed_value_stays_visible_in_a_list_of_choices() {
        let options =
            super::value_options(&["ffplay -nodisp".to_owned()], "my own command", "auto".into());
        let values: Vec<&str> = options.iter().map(|(value, _)| value.as_ref()).collect();
        assert_eq!(values, ["", "ffplay -nodisp", "my own command"]);
    }

    #[test]
    fn a_list_round_trips() {
        let values = vec!["aa:bb".to_owned(), "cc:dd".to_owned()];
        assert_eq!(split_list(&join_list(&values)), values);
    }

    #[test]
    fn theme_values_map_to_the_three_choices() {
        assert_eq!(super::theme_choice("system"), crate::settings::ThemeChoice::System);
        assert_eq!(super::theme_choice("light"), crate::settings::ThemeChoice::Light);
        assert_eq!(super::theme_choice("dark"), crate::settings::ThemeChoice::Dark);
        assert_eq!(super::theme_choice("nonsense"), crate::settings::ThemeChoice::System);
    }

    #[test]
    fn blank_entries_are_dropped() {
        assert_eq!(split_list(" a , , b "), vec!["a".to_owned(), "b".to_owned()]);
    }
}
