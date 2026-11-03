// SPDX-License-Identifier: GPL-3.0-or-later
//! The secret a receiver is recognized by, and the clients that were let in.
//!
//! The seed is the one thing a receiver cannot invent twice: every identity key derives
//! from it, and a client that paired with a receiver remembers the key, not the name. So
//! the seed is stored, and losing it means every client has to pair again.
//!
//! The client list is the protocol's own answer to "who has paired before". A key in it
//! lets a client verify without a PIN when a PIN is configured; the list is also what the
//! interface shows as the devices that have been here.

use std::path::Path;

use serde_json::Value;

use super::store::{Error, Paths};

/// The identity a receiver runs with.
#[derive(Clone, Debug)]
pub struct Identity {
    seed: [u8; 32],
    clients: Vec<[u8; 32]>,
}

impl Identity {
    /// Reads the seed and the client list, creating the seed on a first run.
    ///
    /// # Errors
    ///
    /// Returns [Error::Io] when the directory cannot be created or the seed cannot be
    /// written. A seed file that exists but is not hexadecimal is replaced rather than
    /// reported: it cannot be used, and a receiver that refuses to start is a worse
    /// answer than one that says so through its log.
    pub fn load(paths: &Paths) -> Result<Self, Error> {
        let seed = match std::fs::read_to_string(paths.seed()) {
            Ok(text) => match parse_hex32(&text) {
                Ok(seed) => seed,
                Err(_) => fresh_seed()?,
            },
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => fresh_seed()?,
            Err(source) => return Err(Error::Io { path: paths.seed(), source }),
        };
        let identity = Self { seed, clients: read_clients(&paths.clients()) };
        identity.save_seed(paths)?;
        Ok(identity)
    }

    /// An identity built from a seed, which is what a test uses.
    pub fn test_identity(seed: [u8; 32]) -> Self {
        Self { seed, clients: Vec::new() }
    }

    /// The 32 bytes every identity key derives from.
    pub fn seed(&self) -> [u8; 32] {
        self.seed
    }

    /// The clients a previous session verified, oldest first.
    pub fn clients(&self) -> &[[u8; 32]] {
        &self.clients
    }

    /// Records a key, and reports whether it was new.
    pub fn remember(&mut self, key: [u8; 32]) -> bool {
        if self.clients.contains(&key) {
            return false;
        }
        self.clients.push(key);
        true
    }

    /// Forgets one key, and reports whether it was known.
    pub fn forget(&mut self, key: &[u8; 32]) -> bool {
        let before = self.clients.len();
        self.clients.retain(|known| known != key);
        self.clients.len() != before
    }

    /// Forgets every client, which is what makes the next session start at the PIN again.
    pub fn forget_all(&mut self) {
        self.clients.clear();
    }

    /// Replaces the seed, keeping the client list.
    ///
    /// The identity is the seed: a new one makes this receiver a different device to every
    /// client, and the keys in the list were derived from the old one. Keeping them is
    /// therefore a choice about a list that no longer means anything, and the caller
    /// decides it.
    ///
    /// # Errors
    ///
    /// Returns [Error::Io] when the seed cannot be written.
    pub fn regenerate(&mut self, paths: &Paths) -> Result<(), Error> {
        self.seed = fresh_seed()?;
        self.save_seed(paths)
    }

    /// Writes the client list.
    ///
    /// # Errors
    ///
    /// Returns [Error::Io] when the file cannot be written.
    pub fn save_clients(&self, paths: &Paths) -> Result<(), Error> {
        let keys: Vec<String> = self.clients.iter().map(|key| hex(key)).collect();
        let text = serde_json::to_string_pretty(&keys).unwrap_or_else(|_| "[]".to_owned());
        super::store::save_text(&paths.clients(), &text)
            .map_err(|source| Error::Io { path: paths.clients(), source })
    }

    /// Writes the seed, readable only by its owner where the platform has such a notion.
    fn save_seed(&self, paths: &Paths) -> Result<(), Error> {
        let text = format!("{}\n", hex(&self.seed));
        super::store::save_text(&paths.seed(), &text)
            .map_err(|source| Error::Io { path: paths.seed(), source })
    }
}

/// The clients a document names, skipping entries that are not keys.
fn read_clients(path: &Path) -> Vec<[u8; 32]> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    let Ok(Value::Array(entries)) = serde_json::from_str::<Value>(&text) else {
        return Vec::new();
    };
    entries.iter().filter_map(Value::as_str).filter_map(|key| parse_hex32(key).ok()).collect()
}

/// 32 random bytes.
fn fresh_seed() -> Result<[u8; 32], Error> {
    let mut seed = [0u8; 32];
    getrandom::fill(&mut seed).map_err(|source| Error::Io {
        path: std::path::PathBuf::from("<random>"),
        source: std::io::Error::other(source),
    })?;
    Ok(seed)
}

/// The bytes as lower-case hexadecimal.
pub fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        text.push(char::from(DIGITS[usize::from(byte >> 4)]));
        text.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    text
}

/// Reads 32 bytes from 64 hexadecimal characters.
///
/// The text is read as bytes: a multi-byte character is not a hexadecimal digit, and
/// slicing a string at two-byte boundaries would panic instead of reporting that.
///
/// # Errors
///
/// Returns a sentence naming what is wrong: the length, a pair that is not hexadecimal, or
/// bytes that are not ASCII at all.
pub fn parse_hex32(text: &str) -> Result<[u8; 32], String> {
    let bytes = text.trim().as_bytes();
    if bytes.len() != 64 {
        return Err(format!("expected 64 hexadecimal characters (32 bytes), got {}", bytes.len()));
    }
    let mut seed = [0u8; 32];
    for (index, byte) in seed.iter_mut().enumerate() {
        let pair = &bytes[index * 2..index * 2 + 2];
        let pair =
            std::str::from_utf8(pair).map_err(|_| "a seed is hexadecimal, so it is ASCII")?;
        *byte = u8::from_str_radix(pair, 16).map_err(|_| format!("{pair:?} is not hexadecimal"))?;
    }
    Ok(seed)
}

#[cfg(test)]
mod tests {
    use super::{Identity, hex, parse_hex32};

    #[test]
    fn hexadecimal_round_trips() {
        let bytes = [0x5au8; 32];
        assert_eq!(parse_hex32(&hex(&bytes)), Ok(bytes));
    }

    #[test]
    fn a_short_seed_is_reported() {
        assert!(parse_hex32("1234").is_err());
    }

    #[test]
    fn remembering_is_idempotent() {
        let mut identity = Identity { seed: [0; 32], clients: Vec::new() };
        assert!(identity.remember([1; 32]));
        assert!(!identity.remember([1; 32]));
        assert_eq!(identity.clients().len(), 1);
        assert!(identity.forget(&[1; 32]));
        assert!(!identity.forget(&[1; 32]));
    }
}
