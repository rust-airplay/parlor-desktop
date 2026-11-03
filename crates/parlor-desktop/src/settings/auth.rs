// SPDX-License-Identifier: GPL-3.0-or-later
//! Who may pair, and who may start a session.

use serde::{Deserialize, Serialize};

use super::value::Problem;

/// How a client proves it may connect.
///
/// The choice is the protocol's, not this program's: a PIN can be asked for once, fixed to
/// four digits, or turned off, and each of those is a different answer to "who is allowed
/// in".
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PinChoice {
    /// A PIN drawn when a client pairs, and forgotten once pairing succeeds.
    #[default]
    OneTime,
    /// A PIN the user chooses, used until it is changed.
    Fixed,
    /// No PIN; a client pairs through the legacy route and is remembered.
    Disabled,
}

/// The authentication and client policy of a session.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default)]
pub struct AuthSettings {
    /// Which kind of PIN a client is asked for.
    pub pin: PinChoice,
    /// The four digits of a fixed PIN.
    pub pin_digits: u16,
    /// A digest password a client must give, if any.
    pub password: String,
    /// Whether only the allowed clients may start a session.
    pub restrict: bool,
    /// Device identifiers admitted when [restrict](Self::restrict) is set.
    pub allow: Vec<String>,
    /// Device identifiers never admitted.
    pub block: Vec<String>,
    /// Whether keys verified by an earlier session are remembered.
    ///
    /// Remembering is what lets a client reconnect without pairing again; forgetting
    /// means every session starts at the PIN.
    pub remember_clients: bool,
}

impl Default for AuthSettings {
    fn default() -> Self {
        Self {
            pin: PinChoice::OneTime,
            pin_digits: 1234,
            password: String::new(),
            restrict: false,
            allow: Vec::new(),
            block: Vec::new(),
            remember_clients: true,
        }
    }
}

impl AuthSettings {
    /// The highest PIN the protocol stores.
    pub const MAX_PIN: u16 = 9999;

    /// The PIN mode the receiver is started with.
    pub fn pin_mode(&self) -> rust_airplay::server::PinMode {
        match self.pin {
            PinChoice::OneTime => rust_airplay::server::PinMode::OneTime,
            PinChoice::Fixed => rust_airplay::server::PinMode::Fixed(self.pin_digits % 10_000),
            PinChoice::Disabled => rust_airplay::server::PinMode::Disabled,
        }
    }

    /// The policy the receiver applies in its SETUP handler.
    pub fn client_policy(&self) -> rust_airplay::server::ClientPolicy {
        rust_airplay::server::ClientPolicy {
            restrict: self.restrict,
            allow: trimmed(&self.allow),
            block: trimmed(&self.block),
        }
    }

    /// The password the receiver requires, when one was set.
    pub fn password(&self) -> Option<String> {
        let password = self.password.trim();
        if password.is_empty() { None } else { Some(password.to_owned()) }
    }

    /// The problems with these values, if any.
    pub fn problems(&self) -> Vec<Problem> {
        let mut problems = Vec::new();
        if self.pin == PinChoice::Fixed && self.pin_digits > Self::MAX_PIN {
            problems.push(Problem::new("auth", "pin_digits", "a PIN is at most four digits"));
        }
        if self.restrict && self.allow.is_empty() {
            problems.push(Problem::new(
                "auth",
                "allow",
                "a restricted receiver needs at least one admitted device",
            ));
        }
        problems
    }
}

/// The identifiers in a list, without the blank lines an editor leaves behind.
fn trimmed(identifiers: &[String]) -> Vec<String> {
    identifiers
        .iter()
        .map(|identifier| identifier.trim())
        .filter(|identifier| !identifier.is_empty())
        .map(str::to_owned)
        .collect()
}
