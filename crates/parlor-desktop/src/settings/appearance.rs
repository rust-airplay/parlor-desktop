// SPDX-License-Identifier: GPL-3.0-or-later
//! The window: which theme, and which language.

use serde::{Deserialize, Serialize};

/// What a person wants the window to follow.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ThemeChoice {
    /// Follow the operating system, and change when it does.
    #[default]
    System,
    /// Always light.
    Light,
    /// Always dark.
    Dark,
}

/// The language every label is written in.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum LanguageChoice {
    /// Follow the operating system's language, falling back to English.
    System,
    /// English.
    English,
    /// Simplified Chinese.
    Chinese,
}

/// The appearance and language of the application.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default)]
pub struct Appearance {
    /// Which theme to show.
    pub theme: ThemeChoice,
    /// Which language to write labels in.
    pub language: LanguageChoice,
}

impl Default for Appearance {
    fn default() -> Self {
        Self { theme: ThemeChoice::System, language: LanguageChoice::English }
    }
}
