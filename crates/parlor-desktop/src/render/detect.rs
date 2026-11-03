// SPDX-License-Identifier: GPL-3.0-or-later
//! Finding the programs and libraries each backend needs.
//!
//! Detection answers one question - can this machine play through this backend - and
//! answers it without starting a session: a program is run for its version, a directory is
//! looked in, a plugin is looked for beside the executable. Nothing here loads a backend.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::settings::Settings;
use crate::settings::store::Paths;

/// Whether a backend can be used on this machine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Availability {
    /// It can be used, and this is what it says it is.
    Ready {
        /// The version the program or library reports, when it reports one.
        version: String,
        /// Where the program or library was found.
        location: PathBuf,
    },
    /// A required part is absent.
    Missing {
        /// Which part is absent, because the two answers differ: one is installed and the
        /// other is built.
        part: MissingPart,
        /// What to tell a person about the absence.
        reason: String,
    },
}

/// The part of a backend that is not on this machine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MissingPart {
    /// The application's plugin was not built or is not beside the executable.
    Plugin,
    /// The programs or the runtime were not installed.
    Runtime,
}

impl Availability {
    /// Whether a session can start with this backend.
    pub fn is_ready(&self) -> bool {
        matches!(self, Self::Ready { .. })
    }
}

/// What both backends offer on this machine.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Backends {
    /// The GStreamer backend.
    pub gstreamer: Availability,
    /// The FFmpeg backend.
    pub ffmpeg: Availability,
}

impl Backends {
    /// Whether any backend can play.
    pub fn any_ready(&self) -> bool {
        self.gstreamer.is_ready() || self.ffmpeg.is_ready()
    }
}

/// Looks for both backends.
pub fn probe(settings: &Settings, paths: &Paths) -> Backends {
    Backends { gstreamer: gstreamer(settings, paths), ffmpeg: ffmpeg(settings, paths) }
}

/// Looks for the GStreamer runtime and its video launcher.
pub fn gstreamer(settings: &Settings, paths: &Paths) -> Availability {
    let Some(plugin) = plugin_path() else {
        return Availability::Missing {
            part: MissingPart::Plugin,
            reason: "the GStreamer plugin is not beside the application".to_owned(),
        };
    };
    if !plugin.is_file() {
        return Availability::Missing {
            part: MissingPart::Plugin,
            reason: format!("{} is missing", plugin.display()),
        };
    }
    let Some(launch) = gst_launch(settings, paths) else {
        return Availability::Missing {
            part: MissingPart::Runtime,
            reason: "gst-launch-1.0 was not found".to_owned(),
        };
    };
    let version = version_of(&launch).unwrap_or_else(|| "GStreamer".to_owned());
    Availability::Ready { version, location: launch }
}

/// Looks for the FFmpeg programs a session needs.
pub fn ffmpeg(settings: &Settings, paths: &Paths) -> Availability {
    let Some(programs) = ffmpeg_programs(settings, paths) else {
        return Availability::Missing {
            part: MissingPart::Runtime,
            reason: "ffmpeg and ffplay were not found".to_owned(),
        };
    };
    let version = version_of(&programs.decoder).unwrap_or_else(|| "FFmpeg".to_owned());
    Availability::Ready { version, location: programs.decoder }
}

/// The two programs and two commands the FFmpeg backend needs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FfmpegPrograms {
    /// The decoder, which is ffmpeg itself.
    pub decoder: PathBuf,
    /// The program that plays audio.
    pub player: PathBuf,
    /// The audio sink command, with its arguments.
    pub audio_sink: Vec<String>,
    /// The video sink command, with its arguments.
    pub video_sink: Vec<String>,
}

/// Finds the FFmpeg programs, or None when one is absent.
///
/// The decoder is looked for where it was named, then in the directory the application
/// installs into, then on the PATH. The player is looked for beside the decoder first,
/// because an installation keeps them together: a machine may have ffmpeg from a package
/// manager and ffplay from nowhere at all.
pub fn ffmpeg_programs(settings: &Settings, paths: &Paths) -> Option<FfmpegPrograms> {
    let installed = installed_directory(paths, "ffmpeg");
    let _ = &settings.backend.ffmpeg.decoder;
    let decoder = find_program(&settings.backend.ffmpeg.decoder, &installed)
        .or_else(|| find_program("ffmpeg", &installed))?;
    let mut beside = decoder.parent().map(Path::to_path_buf).into_iter().collect::<Vec<_>>();
    beside.extend(installed.iter().cloned());
    let player = find_program("ffplay", &beside)?;
    let audio_sink = sink_command(&settings.backend.ffmpeg.audio_sink, &player, &["-nodisp"]);
    let video_sink = sink_command(&settings.backend.ffmpeg.video_sink, &player, &["-framedrop"]);
    Some(FfmpegPrograms { decoder, player, audio_sink, video_sink })
}

/// Splits a sink command line, resolving a bare program name against the player.
fn sink_command(named: &str, player: &Path, defaults: &[&str]) -> Vec<String> {
    let mut words = named.split_whitespace().map(str::to_owned);
    let Some(program) = words.next() else {
        let mut command = vec![player.to_string_lossy().into_owned()];
        command.extend(defaults.iter().map(|argument| (*argument).to_owned()));
        return command;
    };
    let mut command = vec![if program.contains(['/', '\\']) {
        program
    } else if program.eq_ignore_ascii_case("ffplay") {
        player.to_string_lossy().into_owned()
    } else {
        program
    }];
    command.extend(words);
    command
}

/// The directory one backend was installed into, if any.
pub fn installed_directory(paths: &Paths, name: &str) -> Vec<PathBuf> {
    vec![paths.backend(name).join("bin"), paths.backend(name)]
}

/// Finds gst-launch-1.0, which the GStreamer backend runs for video.
pub fn gst_launch(settings: &Settings, paths: &Paths) -> Option<PathBuf> {
    if let Some(launch) = find_program(&settings.backend.gstreamer.launch_path, &[]) {
        return Some(launch);
    }
    let directories = gstreamer_directories(paths);
    find_program("gst-launch-1.0", &directories)
}

/// The directories a GStreamer runtime can live in.
///
/// The order matters: a directory the application installed into is preferred over one an
/// operating system's package manager put on the PATH, because the first is the runtime
/// the plugin was built against.
pub fn gstreamer_directories(paths: &Paths) -> Vec<PathBuf> {
    let mut directories = vec![paths.backend("gstreamer").join("bin")];
    if let Some(bundled) =
        std::env::current_exe().ok().and_then(|exe| exe.parent().map(Path::to_path_buf))
    {
        directories.push(bundled.join("gstreamer").join("bin"));
    }
    if let Some(named) = std::env::var_os("PARLOR_GSTREAMER_DIR") {
        directories.push(PathBuf::from(named).join("bin"));
        directories
            .push(PathBuf::from(std::env::var_os("PARLOR_GSTREAMER_DIR").unwrap_or_default()));
    }
    directories
}

/// The location the GStreamer plugin is expected at.
pub fn plugin_path() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    Some(exe.parent()?.join(parlor_backend_api::plugin_file_name()))
}

/// Finds a program by name in these directories, and then on the PATH.
///
/// A name with a separator in it is a path and is used as one. Otherwise the directories
/// are tried in order, then every directory on the PATH; on Windows each candidate is also
/// tried with an executable extension.
pub fn find_program(named: &str, directories: &[PathBuf]) -> Option<PathBuf> {
    let named = named.trim();
    if named.is_empty() {
        return None;
    }
    if named.contains(['/', '\\']) {
        let path = PathBuf::from(named);
        return path.is_file().then_some(path);
    }
    for directory in directories {
        if let Some(path) = candidate(directory, named) {
            return Some(path);
        }
    }
    let path = std::env::var_os("PATH")?;
    for directory in std::env::split_paths(&path) {
        if let Some(path) = candidate(&directory, named) {
            return Some(path);
        }
    }
    None
}

/// One candidate file name in one directory.
fn candidate(directory: &Path, named: &str) -> Option<PathBuf> {
    let direct = directory.join(named);
    if direct.is_file() {
        return Some(direct);
    }
    if cfg!(windows) && Path::new(named).extension().is_none() {
        for extension in ["exe", "cmd", "bat"] {
            let path = directory.join(format!("{named}.{extension}"));
            if path.is_file() {
                return Some(path);
            }
        }
    }
    None
}

/// The first line a program prints about itself, without its banner noise.
pub fn version_of(program: &Path) -> Option<String> {
    for argument in ["--version", "-version"] {
        let Ok(output) = Command::new(program)
            .arg(argument)
            .stdin(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .output()
        else {
            continue;
        };
        if !output.status.success() {
            continue;
        }
        let text = String::from_utf8_lossy(&output.stdout);
        let first = text.lines().next().map(|line| line.trim().to_owned());
        if let Some(first) = first.filter(|line| !line.is_empty()) {
            return Some(first);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{candidate, find_program};
    use std::path::Path;

    #[test]
    fn an_empty_name_is_not_a_program() {
        assert_eq!(find_program("", &[]), None);
    }

    #[test]
    fn a_program_is_found_in_a_directory() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let program = directory.path().join(if cfg!(windows) { "probe.exe" } else { "probe" });
        std::fs::write(&program, b"").expect("a file");
        let found = find_program("probe", &[directory.path().to_path_buf()]);
        assert_eq!(found.as_deref(), Some(program.as_path()));
        assert!(candidate(Path::new("."), "nothing-here").is_none());
    }
}
