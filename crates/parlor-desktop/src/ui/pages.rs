// SPDX-License-Identifier: GPL-3.0-or-later
//! The settings pages, one page per group of choices.
//!
//! Every control here writes one field of [Settings](crate::settings::Settings) and saves
//! it. The receiver reads its configuration once when a session starts, so a page says
//! that a change takes effect next time, and there is nothing else to press.

use gpui_kit::Axis;
use gpui_kit::component::ActiveTheme as _;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::setting::{
    NumberFieldOptions, SettingField, SettingGroup, SettingItem, SettingPage, Settings,
};
use gpui_kit::component::{Sizable, Theme, ThemeMode, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::i18n::{Lang, Text};
use crate::render::detect::{Availability, MissingPart};
use crate::render::install::{self, Progress, Target};
use crate::settings::{
    AudioCodecChoice, BackendKind, DisplaySettings, LanguageChoice, LogLevel, PinChoice, PortsMode,
    Settings as Chosen, ThemeChoice, VideoCodecChoice,
};
use crate::state;
use crate::ui::RootView;

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
        appearance(cx),
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
    SettingPage::new(text(Text::SettingsGeneral))
        .description(text(Text::SettingsDescription))
        .group(SettingGroup::new().title(text(Text::SessionTitle)).items(vec![
                SettingItem::new(
                    text(Text::ReceiverName),
                    SettingField::input(
                        |cx| {
                            SharedString::from(
                                cx.global::<state::Parlor>().settings.receiver.name.clone(),
                            )
                        },
                        |value: SharedString, cx| {
                            state::update_settings(cx, |settings| {
                                settings.receiver.name = value.to_string();
                            });
                        },
                    ),
                ),
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

/// The theme and the language.
fn appearance(cx: &App) -> SettingPage {
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
    SettingPage::new(text(Text::SettingsAppearance)).group(
        SettingGroup::new().title(text(Text::Theme)).items(vec![
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
        ]),
    )
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
                SettingField::input(
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
    let pin = match cx.global::<state::Parlor>().settings.auth.pin {
        PinChoice::OneTime => "one-time",
        PinChoice::Fixed => "fixed",
        PinChoice::Disabled => "disabled",
    };
    let mut page = SettingPage::new(text(Text::SettingsSecurity))
        .group(SettingGroup::new().title(text(Text::Pairing)).items(vec![
                SettingItem::new(
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
                ),
                SettingItem::new(
                    text(Text::Password),
                    SettingField::input(
                        |cx| {
                            SharedString::from(
                                cx.global::<state::Parlor>().settings.auth.password.clone(),
                            )
                        },
                        |value: SharedString, cx| {
                            state::update_settings(cx, |settings| {
                                settings.auth.password = value.to_string();
                            });
                        },
                    ),
                )
                .layout(Axis::Vertical).description(text(Text::PasswordHint)),
                SettingItem::new(
                    text(Text::RememberClients),
                    SettingField::switch(
                        |cx| cx.global::<state::Parlor>().settings.auth.remember_clients,
                        |value, cx| {
                            state::update_settings(cx, |settings| {
                                settings.auth.remember_clients = value
                            });
                        },
                    ),
                )
                .layout(Axis::Vertical).description(text(Text::RememberClientsHint)),
            ]))
        .group(SettingGroup::new().title(text(Text::AllowedDevices)).items(vec![
                SettingItem::new(
                    text(Text::RestrictClients),
                    SettingField::switch(
                        |cx| cx.global::<state::Parlor>().settings.auth.restrict,
                        |value, cx| {
                            state::update_settings(cx, |settings| settings.auth.restrict = value);
                        },
                    ),
                )
                .layout(Axis::Vertical).description(text(Text::RestrictClientsHint)),
                SettingItem::new(
                    text(Text::BlockedDevices),
                    SettingField::input(
                        |cx| {
                            SharedString::from(join_list(
                                &cx.global::<state::Parlor>().settings.auth.block,
                            ))
                        },
                        |value: SharedString, cx| {
                            state::update_settings(cx, |settings| {
                                settings.auth.block = split_list(&value);
                            });
                        },
                    ),
                ),
            ]))
        .group(SettingGroup::new().title(text(Text::KnownDevices)).item(known_devices(lang)));
    if pin == "fixed" {
        page = page.group(SettingGroup::new().title(text(Text::PinDigits)).item(SettingItem::new(
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
        )));
    }
    if cx.global::<state::Parlor>().settings.auth.restrict {
        page = page.group(
            SettingGroup::new().title(text(Text::AllowedDevices)).item(
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
                                settings.auth.allow = split_list(&value);
                            });
                        },
                    ),
                )
                .layout(Axis::Vertical)
                .description(text(Text::OnePerLine)),
            ),
        );
    }
    page
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
                text(Text::Width),
                SettingField::number_input(
                    NumberFieldOptions {
                        min: f64::from(DisplaySettings::MIN_SIDE),
                        max: f64::from(DisplaySettings::MAX_SIDE),
                        step: 2.0,
                    },
                    |cx| f64::from(cx.global::<state::Parlor>().settings.display.width),
                    |value, cx| {
                        state::update_settings(cx, |settings| {
                            settings.display.width = value.round().clamp(
                                f64::from(DisplaySettings::MIN_SIDE),
                                f64::from(DisplaySettings::MAX_SIDE),
                            ) as u32;
                        });
                    },
                ),
            ),
            SettingItem::new(
                text(Text::Height),
                SettingField::number_input(
                    NumberFieldOptions {
                        min: f64::from(DisplaySettings::MIN_SIDE),
                        max: f64::from(DisplaySettings::MAX_SIDE),
                        step: 2.0,
                    },
                    |cx| f64::from(cx.global::<state::Parlor>().settings.display.height),
                    |value, cx| {
                        state::update_settings(cx, |settings| {
                            settings.display.height = value.round().clamp(
                                f64::from(DisplaySettings::MIN_SIDE),
                                f64::from(DisplaySettings::MAX_SIDE),
                            ) as u32;
                        });
                    },
                ),
            ),
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
            ),
            SettingItem::new(
                text(Text::RefreshRate),
                SettingField::number_input(
                    NumberFieldOptions { min: 1.0, max: 240.0, step: 1.0 },
                    |cx| cx.global::<state::Parlor>().settings.display.refresh_rate,
                    |value, cx| {
                        state::update_settings(cx, |settings| {
                            settings.display.refresh_rate = value.clamp(1.0, 240.0);
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

/// Which backend plays, and how.
fn backend(cx: &App, install: Option<(Target, Progress)>) -> SettingPage {
    let lang = state::lang(cx);
    let text = move |label: Text| label.of(lang);
    let parlor = state::parlor(cx);
    let backends = parlor.backends.clone();
    let paths = parlor.paths.clone();
    let kind = parlor.settings.backend.kind;
    let mut groups = vec![SettingGroup::new().title(text(Text::BackendKind)).items(vec![
        SettingItem::new(
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
        ),
        availability_item(text, lang, Target::GStreamer, backends.gstreamer.clone(), paths.clone()),
        availability_item(text, lang, Target::Ffmpeg, backends.ffmpeg.clone(), paths.clone()),
        SettingItem::render(move |_, _window, cx| {
            let lang = state::lang(cx);
            let text = move |label: Text| label.of(lang);
            let folder = std::env::current_exe()
                .ok()
                .and_then(|executable| executable.parent().map(std::path::Path::to_path_buf));
            v_flex()
                .gap_1()
                .items_start()
                .child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(text(Text::BackendSearchHint)),
                )
                .when_some(folder, |this, folder| {
                    this.child(
                        Button::new("open-application-folder")
                            .xsmall()
                            .outline()
                            .label(text(Text::OpenApplicationFolder))
                            .on_click(move |_, _, _| {
                                let _ = std::process::Command::new("explorer").arg(&folder).spawn();
                            }),
                    )
                })
        }),
    ])];
    if let Some((target, progress)) = install {
        groups.push(SettingGroup::new().title(text(Text::InstallTitle)).item(SettingItem::render(
            move |_, _window, cx| {
                let text = |label: Text| label.of(state::lang(cx));
                v_flex()
                    .gap_2()
                    .child(div().text_sm().child(match target {
                        Target::Ffmpeg => text(Text::Ffmpeg),
                        Target::GStreamer => text(Text::GStreamer),
                    }))
                    .child(format!("{:.0}%", progress.fraction() * 100.0))
            },
        )));
    }
    match kind {
        BackendKind::Ffmpeg => groups.push(ffmpeg_group(cx)),
        BackendKind::GStreamer => groups.push(gstreamer_group(cx)),
    }
    SettingPage::new(text(Text::SettingsBackend)).groups(groups)
}

/// One backend's availability, with the buttons that change it.
fn availability_item(
    _text: impl Fn(Text) -> &'static str + Copy + 'static,
    _lang: Lang,
    target: Target,
    availability: Availability,
    paths: crate::settings::store::Paths,
) -> SettingItem {
    SettingItem::render(move |_, _window, cx| {
        let lang = state::lang(cx);
        let text = move |label: Text| label.of(lang);
        let (summary, colour) = match &availability {
            Availability::Ready { version, .. } => {
                (format!("{} {version}", text(Text::Available)), cx.theme().success)
            }
            Availability::Missing { part, .. } => (
                match part {
                    MissingPart::Plugin => text(Text::PluginMissing).to_owned(),
                    MissingPart::Runtime => text(Text::NotInstalled).to_owned(),
                },
                cx.theme().muted_foreground,
            ),
        };
        let present = install::installed(target, &paths);
        let installable =
            !matches!(availability, Availability::Missing { part: MissingPart::Plugin, .. });
        let location = match &availability {
            Availability::Ready { location, .. } => Some(location.display().to_string()),
            Availability::Missing { .. } => None,
        };
        let sender = cx.global::<state::Parlor>().installs.clone();
        let remove_paths = paths.clone();
        let install_sender = sender.clone();
        let reinstall_sender = sender.clone();
        h_flex()
            .gap_2()
            .items_center()
            .child(div().text_xs().text_color(colour).child(summary))
            .when_some(location, |this, location| {
                this.child(div().text_xs().text_color(cx.theme().muted_foreground).child(location))
            })
            .when(!present && installable, |this| {
                this.child(
                    Button::new(SharedString::from(format!("install-{}", target.directory())))
                        .xsmall()
                        .primary()
                        .label(text(Text::InstallNow))
                        .on_click(move |_, _, cx| {
                            let _ = install_sender.try_send(target);
                            cx.refresh_windows();
                        }),
                )
            })
            .when(present, |this| {
                this.child(
                    Button::new(SharedString::from(format!("reinstall-{}", target.directory())))
                        .xsmall()
                        .outline()
                        .label(text(Text::Reinstall))
                        .on_click(move |_, _, cx| {
                            let _ = reinstall_sender.try_send(target);
                            cx.refresh_windows();
                        }),
                )
                .child(
                    Button::new(SharedString::from(format!("remove-{}", target.directory())))
                        .xsmall()
                        .label(text(Text::Uninstall))
                        .on_click(move |_, _, cx| {
                            let _ = install::remove(target, &remove_paths);
                            state::reprobe(cx);
                        }),
                )
            })
    })
}

/// The FFmpeg backend's own choices.
///
/// The coding a warm sink starts for is shown only while a warm sink is started: with
/// nothing kept alive there is no coding to choose.
fn ffmpeg_group(cx: &App) -> SettingGroup {
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
            SettingField::input(
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
        .layout(Axis::Vertical)
        .description(text(Text::EmptyUsesBackendDefault)),
        SettingItem::new(
            text(Text::AudioSink),
            SettingField::input(
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
            SettingField::input(
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
        .layout(Axis::Vertical)
        .description(text(Text::WarmVideoHint)),
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
    SettingGroup::new().title(text(Text::Ffmpeg)).items(items)
}

/// The GStreamer backend's own choices.
fn gstreamer_group(cx: &App) -> SettingGroup {
    let lang = state::lang(cx);
    let text = move |label: Text| label.of(lang);
    let input_item =
        |label: &'static str, get: fn(&Chosen) -> String, set: fn(&mut Chosen) -> &mut String| {
            let _ = label;
            SettingItem::new(
                label,
                SettingField::input(
                    move |cx| SharedString::from(get(&cx.global::<state::Parlor>().settings)),
                    move |value: SharedString, cx| {
                        state::update_settings(cx, |settings| {
                            *set(settings) = value.to_string();
                        });
                    },
                ),
            )
            .layout(Axis::Vertical)
            .description(text(Text::EmptyUsesBackendDefault))
        };
    SettingGroup::new().title(text(Text::GStreamer)).items(vec![
        input_item(
            text(Text::Location),
            |settings| settings.backend.gstreamer.launch_path.clone(),
            |settings| &mut settings.backend.gstreamer.launch_path,
        ),
        input_item(
            text(Text::AudioSink),
            |settings| settings.backend.gstreamer.audio_sink.clone(),
            |settings| &mut settings.backend.gstreamer.audio_sink,
        ),
        input_item(
            text(Text::VideoSink),
            |settings| settings.backend.gstreamer.video_sink.clone(),
            |settings| &mut settings.backend.gstreamer.video_sink,
        ),
        input_item(
            text(Text::H264Parser),
            |settings| settings.backend.gstreamer.h264_parser.clone(),
            |settings| &mut settings.backend.gstreamer.h264_parser,
        ),
        input_item(
            text(Text::H264Decoder),
            |settings| settings.backend.gstreamer.h264_decoder.clone(),
            |settings| &mut settings.backend.gstreamer.h264_decoder,
        ),
        input_item(
            text(Text::H265Parser),
            |settings| settings.backend.gstreamer.h265_parser.clone(),
            |settings| &mut settings.backend.gstreamer.h265_parser,
        ),
        input_item(
            text(Text::H265Decoder),
            |settings| settings.backend.gstreamer.h265_decoder.clone(),
            |settings| &mut settings.backend.gstreamer.h265_decoder,
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
        .layout(Axis::Vertical)
        .description(text(Text::WarmVideoHint)),
    ])
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
    SettingPage::new(text(Text::SettingsDiagnostics))
        .group(SettingGroup::new().title(text(Text::Log)).items(vec![
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
        ]))
        .group(SettingGroup::new().title(text(Text::Log)).items(vec![
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
        ]))
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
            SettingItem::new(
                text(Text::Version),
                SettingField::input(|_| SharedString::from(env!("CARGO_PKG_VERSION")), |_, _| {}),
            ),
            SettingItem::new(
                text(Text::License),
                SettingField::input(
                    move |_| SharedString::from(text(Text::LicenseBody)),
                    |_, _| {},
                ),
            ),
            SettingItem::render(move |_, _window, cx| {
                let text = |label: Text| label.of(state::lang(cx));
                h_flex().gap_2().child(
                    Button::new("homepage").xsmall().label(text(Text::Homepage)).on_click(
                        |_, _, cx| {
                            cx.open_url("https://github.com/rust-airplay");
                        },
                    ),
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

#[cfg(test)]
mod tests {
    use super::{join_list, split_list};

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
