// SPDX-License-Identifier: GPL-3.0-or-later
//! Whether a newer release exists.
//!
//! The repository keeps the version of its latest release in a Version file, and the about
//! page reads it: the alternative, asking the GitHub API, needs a rate limit budget and a
//! user agent, while one small file is a single request any machine can make.
//!
//! A release is installed by replacing the program, so this module only reports what the
//! network says; the about page offers the downloads page and leaves the replacement to
//! the person.

use std::time::Duration;

/// The file that names the version of the latest release.
pub const VERSION_URL: &str =
    "https://raw.githubusercontent.com/rust-airplay/parlor-desktop/main/Version";

/// Where a person downloads a release.
pub const RELEASES_URL: &str = "https://github.com/rust-airplay/parlor-desktop/releases/latest";

/// The version this build carries.
pub const CURRENT: &str = env!("CARGO_PKG_VERSION");

/// What the about page knows about a newer release.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UpdateState {
    /// Nobody has asked yet.
    Idle,
    /// A request is in flight.
    Checking,
    /// The release file names this build or an older one.
    UpToDate,
    /// A newer release exists, with its version.
    Available(String),
    /// The question could not be answered.
    Failed(String),
}

/// Asks the repository which version is current.
///
/// The request runs on its own thread: the window must keep drawing while a network that
/// is slow, or absent, spends its timeout.
pub fn check() -> UpdateState {
    match latest() {
        Ok(Some(version)) => UpdateState::Available(version),
        Ok(None) => UpdateState::UpToDate,
        Err(reason) => UpdateState::Failed(reason),
    }
}

/// The version of the latest release, when it is newer than this build.
fn latest() -> Result<Option<String>, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|error| error.to_string())?;
    let response = client
        .get(VERSION_URL)
        .header(reqwest::header::CACHE_CONTROL, "no-cache")
        .send()
        .map_err(|error| error.to_string())?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("HTTP {status}"));
    }
    let body = response.text().map_err(|error| error.to_string())?;
    let version = body.trim();
    if version.is_empty() {
        return Err("the version file is empty".to_owned());
    }
    Ok(is_newer(version, CURRENT).then(|| version.to_owned()))
}

/// Whether the latest version is after the current one.
///
/// A version that is not three numbers is compared as text, which is at least stable while
/// a maintainer decides on a scheme.
fn is_newer(latest: &str, current: &str) -> bool {
    match (version_of(latest), version_of(current)) {
        (Some(latest), Some(current)) => latest > current,
        _ => latest.trim() != current.trim(),
    }
}

/// The first three numbers of a version, ignoring a leading v and any suffix.
fn version_of(text: &str) -> Option<(u64, u64, u64)> {
    let text = text.trim().trim_start_matches(['v', 'V']);
    let mut parts = text.split('.');
    let major = number(parts.next()?)?;
    let minor = number(parts.next().unwrap_or("0"))?;
    let patch = number(parts.next().unwrap_or("0"))?;
    Some((major, minor, patch))
}

/// The number a part begins with, so a suffix after a dash is ignored.
fn number(part: &str) -> Option<u64> {
    let end = part.find(|character: char| !character.is_ascii_digit()).unwrap_or(part.len());
    part[..end].parse().ok()
}

#[cfg(test)]
mod tests {
    use super::{is_newer, version_of};

    #[test]
    fn a_version_is_three_numbers() {
        assert_eq!(version_of("0.1.0"), Some((0, 1, 0)));
        assert_eq!(version_of(" v1.2.3 "), Some((1, 2, 3)));
        assert_eq!(version_of("1.2"), Some((1, 2, 0)));
        assert_eq!(version_of("1.2.3-beta.1"), Some((1, 2, 3)));
        assert_eq!(version_of("nightly"), None);
    }

    #[test]
    fn only_a_later_version_counts() {
        assert!(is_newer("0.2.0", "0.1.0"));
        assert!(is_newer("v0.1.1", "0.1.0"));
        assert!(!is_newer("0.1.0", "0.1.0"));
        assert!(!is_newer("0.0.9", "0.1.0"));
    }
}
