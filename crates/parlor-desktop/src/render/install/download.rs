// SPDX-License-Identifier: GPL-3.0-or-later
//! Reading a file from the network, with a way to stop.

use std::io::{Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// Downloads a URL to a file, reporting how many bytes have arrived.
///
/// The read is streamed rather than buffered: a backend archive is a hundred megabytes,
/// and a program that holds one in memory to write it out again is a program that can be
/// killed by a machine with less memory than that.
///
/// # Errors
///
/// Returns a sentence when the request fails, the server refuses, the file cannot be
/// written, or [cancelled] is set while reading.
pub fn to_file(
    url: &str,
    destination: &Path,
    cancelled: &AtomicBool,
    mut progress: impl FnMut(u64, Option<u64>),
) -> Result<(), String> {
    let client = reqwest::blocking::Client::builder()
        .connect_timeout(Duration::from_secs(30))
        .user_agent(concat!("Parlor/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|error| format!("cannot create an HTTP client: {error}"))?;
    let mut response =
        client.get(url).send().map_err(|error| format!("cannot reach {url}: {error}"))?;
    if !response.status().is_success() {
        return Err(format!("{url} answered {}", response.status()));
    }
    let total = response.content_length();
    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    }
    let mut file = std::fs::File::create(destination)
        .map_err(|error| format!("cannot write {}: {error}", destination.display()))?;
    let mut buffer = [0u8; 64 * 1024];
    let mut received = 0u64;
    loop {
        if cancelled.load(Ordering::Relaxed) {
            return Err("the installation was cancelled".to_owned());
        }
        let read =
            response.read(&mut buffer).map_err(|error| format!("reading {url} failed: {error}"))?;
        if read == 0 {
            break;
        }
        file.write_all(&buffer[..read])
            .map_err(|error| format!("writing {} failed: {error}", destination.display()))?;
        received += u64::try_from(read).unwrap_or(0);
        progress(received, total);
    }
    Ok(())
}
