// SPDX-License-Identifier: GPL-3.0-or-later
//! What is wrong with a setting, said once and shown where it was written.

use serde::{Deserialize, Serialize};

/// One field that cannot be used, and why.
///
/// The field is named by its settings page and its key rather than by a widget, because
/// the same problem is shown on a page, in a dialog that offers to fix it, and in a log
/// line; a widget identifier would be different in each of the three.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct Problem {
    /// The settings page that holds the field, such as "network".
    pub page: &'static str,
    /// The field within that page, such as "port".
    pub field: &'static str,
    /// What a person has to change, in one sentence.
    pub message: String,
}

impl Problem {
    /// A problem with one field on one page.
    pub fn new(page: &'static str, field: &'static str, message: impl Into<String>) -> Self {
        Self { page, field, message: message.into() }
    }
}

impl std::fmt::Display for Problem {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.field, self.message)
    }
}
