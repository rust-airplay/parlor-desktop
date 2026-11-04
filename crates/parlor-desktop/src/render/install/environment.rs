// SPDX-License-Identifier: GPL-3.0-or-later
//! Recording an installed backend in the environment.
//!
//! Detection looks for a backend in `lib` first and in the environment second. An installation
//! under the application's data directory is therefore not found unless the environment says
//! where it is, which is why installing one writes its variable: the next session, and the
//! next run of the program, find it the same way a person who set the variable by hand would.
//!
//! The variable is written for the current user (HKCU\Environment), so no administrator
//! rights are involved and another user's environment is untouched. Windows broadcasts the
//! change after the write; this process sets its own copy immediately because a running
//! program does not re-read the registry.

#![allow(unsafe_code)] // Writing a user environment variable is a Windows registry call.

use std::path::Path;

use super::Target;

/// The variable a GStreamer installation is recorded in.
pub const GSTREAMER_VARIABLE: &str = "PARLOR_GSTREAMER_DIR";
/// The variable an FFmpeg installation is recorded in.
pub const FFMPEG_VARIABLE: &str = "PARLOR_FFMPEG_DIR";

/// The variable a backend's directory is recorded in.
pub fn variable(target: Target) -> &'static str {
    match target {
        Target::GStreamer => GSTREAMER_VARIABLE,
        Target::Ffmpeg => FFMPEG_VARIABLE,
    }
}

/// Records a backend's directory in the current process and in the user's environment.
///
/// The in-process copy is set first, so this run can use the backend even when the persistent
/// write fails; the returned error is the persistent one.
///
/// # Errors
///
/// Returns a sentence when the environment could not be written. The installation itself is
/// not undone: the files are usable, and the variable can be set by hand.
pub fn remember(target: Target, directory: &Path) -> Result<(), String> {
    let name = variable(target);
    let value = directory.to_string_lossy().into_owned();
    // SAFETY: the receiver threads are not reading this name yet; a session only reads it
    // when a backend is built, and an installation happens before the next session starts.
    unsafe {
        std::env::set_var(name, &value);
    }
    persist(name, &value)
}

/// Removes a backend's directory from the environment, when it is the one recorded.
///
/// # Errors
///
/// Returns a sentence when the environment could not be written.
pub fn forget(target: Target, directory: &Path) -> Result<(), String> {
    let name = variable(target);
    let value = directory.to_string_lossy().into_owned();
    let recorded = std::env::var(name).ok();
    if recorded.as_deref() != Some(value.as_str()) {
        // The variable names something else, which is not this installation to forget.
        if recorded.is_none() {
            return Ok(());
        }
    }
    // SAFETY: a session reads this name only while it builds a backend, and a removal
    // happens between sessions.
    unsafe {
        std::env::remove_var(name);
    }
    clear(name)
}

#[cfg(target_os = "windows")]
mod platform {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;

    use windows_sys::Win32::Foundation::ERROR_SUCCESS;
    use windows_sys::Win32::System::Registry::{
        HKEY, HKEY_CURRENT_USER, KEY_SET_VALUE, REG_SZ, RegCloseKey, RegDeleteValueW,
        RegOpenKeyExW, RegSetValueExW,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        HWND_BROADCAST, SMTO_ABORTIFHUNG, SendMessageTimeoutW, WM_SETTINGCHANGE,
    };

    const ENVIRONMENT: &str = "Environment";

    /// A UTF-16 string with its terminator.
    fn wide(value: &str) -> Vec<u16> {
        OsStr::new(value).encode_wide().chain([0]).collect()
    }

    /// The user's environment key, opened for one kind of access.
    fn open(access: u32) -> Result<HKEY, String> {
        let mut key: HKEY = std::ptr::null_mut();
        // SAFETY: the key name is null-terminated and the handle is written only on success.
        let opened = unsafe {
            RegOpenKeyExW(HKEY_CURRENT_USER, wide(ENVIRONMENT).as_ptr(), 0, access, &mut key)
        };
        if opened != ERROR_SUCCESS {
            return Err(format!("cannot open HKCU\\Environment: Windows error {opened}"));
        }
        Ok(key)
    }

    /// Writes one string value.
    pub fn persist(name: &str, value: &str) -> Result<(), String> {
        let key = open(KEY_SET_VALUE)?;
        let name = wide(name);
        let data: Vec<u16> = OsStr::new(value).encode_wide().chain([0]).collect();
        // SAFETY: the key is open, both strings outlive the call, and the byte count matches
        // the buffer: two bytes per UTF-16 code unit.
        let written = unsafe {
            RegSetValueExW(
                key,
                name.as_ptr(),
                0,
                REG_SZ,
                data.as_ptr().cast(),
                u32::try_from(data.len() * 2).unwrap_or(u32::MAX),
            )
        };
        // SAFETY: the handle came from RegOpenKeyExW and is not used afterwards.
        unsafe {
            RegCloseKey(key);
        }
        if written != ERROR_SUCCESS {
            return Err(format!("cannot write the environment: Windows error {written}"));
        }
        broadcast();
        Ok(())
    }

    /// Deletes one value, if it is there.
    pub fn clear(name: &str) -> Result<(), String> {
        let key = open(KEY_SET_VALUE)?;
        let name = wide(name);
        // SAFETY: the key is open and the name outlives the call.
        let deleted = unsafe { RegDeleteValueW(key, name.as_ptr()) };
        // SAFETY: the handle came from RegOpenKeyExW.
        unsafe {
            RegCloseKey(key);
        }
        if deleted != ERROR_SUCCESS {
            // A value that is not there is the state the caller wanted.
            return Err(format!("cannot clear the environment: Windows error {deleted}"));
        }
        broadcast();
        Ok(())
    }

    /// Reads one value from the user's environment.
    pub fn read(name: &str) -> Result<String, String> {
        use windows_sys::Win32::System::Registry::{KEY_QUERY_VALUE, RegQueryValueExW};

        let key = open(KEY_QUERY_VALUE)?;
        let name = wide(name);
        let mut kind = 0u32;
        let mut bytes = 0u32;
        // SAFETY: the key is open; a null buffer asks for the size only.
        let sized = unsafe {
            RegQueryValueExW(
                key,
                name.as_ptr(),
                std::ptr::null(),
                &mut kind,
                std::ptr::null_mut(),
                &mut bytes,
            )
        };
        if sized != ERROR_SUCCESS || bytes == 0 {
            // SAFETY: the handle came from RegOpenKeyExW.
            unsafe {
                RegCloseKey(key);
            }
            return Err("the variable is not set".to_owned());
        }
        let mut buffer = vec![0u8; bytes as usize];
        // SAFETY: the buffer is as large as the size the first call reported.
        let read = unsafe {
            RegQueryValueExW(
                key,
                name.as_ptr(),
                std::ptr::null(),
                &mut kind,
                buffer.as_mut_ptr(),
                &mut bytes,
            )
        };
        // SAFETY: the handle came from RegOpenKeyExW.
        unsafe {
            RegCloseKey(key);
        }
        if read != ERROR_SUCCESS {
            return Err(format!("cannot read the environment: Windows error {read}"));
        }
        let mut units = Vec::with_capacity(buffer.len() / 2);
        for pair in buffer.chunks(2) {
            if let [low, high] = pair {
                units.push(u16::from_ne_bytes([*low, *high]));
            }
        }
        let text = String::from_utf16_lossy(&units);
        Ok(text.trim_end_matches('\0').to_owned())
    }

    /// Tells the running programs that the environment changed.
    fn broadcast() {
        let mut result = 0usize;
        // SAFETY: the broadcast window handle is a constant, the string outlives the call,
        // and a timeout means a hung window is skipped rather than blocking this one.
        unsafe {
            SendMessageTimeoutW(
                HWND_BROADCAST,
                WM_SETTINGCHANGE,
                0,
                0,
                SMTO_ABORTIFHUNG,
                5_000,
                &mut result,
            );
        }
    }
}

#[cfg(target_os = "windows")]
use platform::{clear, persist};

/// The directory the user's environment names, read from the registry.
///
/// A program started from a shell that predates an installation inherits that shell's copy
/// of the environment, so the process copy can be stale while the installation is perfectly
/// usable. Detection falls back to the registry for exactly that case.
#[cfg(target_os = "windows")]
pub fn recorded(variable: &str) -> Option<std::path::PathBuf> {
    platform::read(variable).ok().filter(|value| !value.is_empty()).map(std::path::PathBuf::from)
}

/// Mirrors [recorded] where there is no registry to read.
#[cfg(not(target_os = "windows"))]
pub fn recorded(_variable: &str) -> Option<std::path::PathBuf> {
    None
}

#[cfg(not(target_os = "windows"))]
fn persist(_name: &str, _value: &str) -> Result<(), String> {
    Err("only Windows writes a user environment".to_owned())
}

#[cfg(not(target_os = "windows"))]
fn clear(_name: &str) -> Result<(), String> {
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn read(_name: &str) -> Result<String, String> {
    Err("only Windows reads a user environment".to_owned())
}

#[cfg(test)]
mod integration_tests {
    use super::{FFMPEG_VARIABLE, GSTREAMER_VARIABLE, Target, forget, remember};
    use crate::render::detect;
    use crate::settings::{BackendKind, Settings};

    #[test]
    #[ignore = "writes the real user environment for the length of the test"]
    fn an_installation_is_found_when_only_the_registry_names_it() {
        if std::env::var(GSTREAMER_VARIABLE).is_ok()
            || super::recorded(GSTREAMER_VARIABLE).is_some()
        {
            return;
        }
        let directory = tempfile::tempdir().expect("a temporary directory");
        let root = directory.path().join("backends").join("gstreamer");
        std::fs::create_dir_all(root.join("bin")).expect("a bin directory");
        std::fs::write(root.join("bin").join("gst-launch-1.0.exe"), b"").expect("a program");
        super::platform::persist(GSTREAMER_VARIABLE, &root.to_string_lossy())
            .expect("a value in the registry");
        // The process copy is deliberately left alone: a shell older than the installation
        // is what the fallback exists for.
        let mut settings = Settings::new();
        settings.backend.kind = BackendKind::GStreamer;
        let found = detect::gst_launch(&settings).expect("a launcher");
        assert!(found.starts_with(root.join("bin")), "{}", found.display());
        super::platform::clear(GSTREAMER_VARIABLE).expect("a forgotten value");
        assert!(super::recorded(GSTREAMER_VARIABLE).is_none(), "the registry no longer names it");
    }

    #[test]
    #[ignore = "writes the real user environment for the length of the test"]
    fn an_installed_launcher_is_found_through_the_environment() {
        if std::env::var(GSTREAMER_VARIABLE).is_ok() {
            return;
        }
        let directory = tempfile::tempdir().expect("a temporary directory");
        let root = directory.path().join("backends").join("gstreamer");
        std::fs::create_dir_all(root.join("bin")).expect("a bin directory");
        std::fs::write(root.join("bin").join("gst-launch-1.0.exe"), b"").expect("a program");
        remember(Target::GStreamer, &root).expect("the variable");
        let mut settings = Settings::new();
        settings.backend.kind = BackendKind::GStreamer;
        let found = detect::gst_launch(&settings).expect("a launcher");
        assert!(found.starts_with(root.join("bin")), "{}", found.display());
        forget(Target::GStreamer, &root).expect("a forgotten variable");
    }

    #[test]
    #[ignore = "writes the real user environment for the length of the test"]
    fn an_installed_decoder_is_found_through_the_environment() {
        if std::env::var(FFMPEG_VARIABLE).is_ok() {
            return;
        }
        let directory = tempfile::tempdir().expect("a temporary directory");
        let root = directory.path().join("backends").join("ffmpeg");
        std::fs::create_dir_all(root.join("bin")).expect("a bin directory");
        std::fs::write(root.join("bin").join("ffmpeg.exe"), b"").expect("a program");
        std::fs::write(root.join("bin").join("ffplay.exe"), b"").expect("a program");
        remember(Target::Ffmpeg, &root).expect("the variable");
        let settings = Settings::new();
        let programs = detect::ffmpeg_programs(&settings).expect("the programs");
        assert!(programs.decoder.starts_with(root.join("bin")), "{}", programs.decoder.display());
        forget(Target::Ffmpeg, &root).expect("a forgotten variable");
    }
}

#[cfg(test)]
mod tests {
    use super::platform::clear;
    use super::platform::persist;
    use super::platform::read;

    #[test]
    fn a_variable_round_trips_through_the_user_environment() {
        let name = "PARLOR_TEST_ENVIRONMENT_VARIABLE";
        let value = r"C:\Program Files\Parlor\backends\ffmpeg";
        persist(name, value).expect("a written variable");
        assert_eq!(read(name).as_deref(), Ok(value));
        clear(name).expect("a cleared variable");
        assert!(read(name).is_err(), "the variable is gone");
    }
}
