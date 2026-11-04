// SPDX-License-Identifier: GPL-3.0-or-later
//! Finding the programs and libraries each backend needs.
//!
//! The search is deliberately short. A backend is either shipped with the application, in
//! the `lib` directory beside the executable, or named by the environment. Nothing is looked
//! for in an installation directory of this program's own, and the PATH is the last answer
//! rather than the first: a machine with a MinGW GStreamer on its PATH has a runtime whose
//! DLLs have the same names as the MSVC one the plugin was built against, and picking that
//! one is a crash rather than a session.
//!
//! ```text
//! explicit setting -> lib beside the executable -> AIRPLAY_* -> PARLOR_* -> PATH
//! ```

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::settings::Settings;

/// The layout a bundled backend is looked for in, under `lib` beside the executable.
///
/// Three shapes are accepted because three are what a person actually produces: the flat
/// copy, a `bin` directory, and a runtime unpacked under its own name.
const BUNDLED_LAYOUT: [&str; 3] = ["lib", "lib/bin", "lib/gstreamer/bin"];

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
        /// Which part is absent, because the two answers differ: the plugin is built and
        /// the runtime is installed.
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
    /// The programs or the runtime were not found.
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
pub fn probe(settings: &Settings) -> Backends {
    Backends { gstreamer: gstreamer(settings), ffmpeg: ffmpeg(settings) }
}

/// Looks for the GStreamer plugin, its runtime and its video launcher.
pub fn gstreamer(settings: &Settings) -> Availability {
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
    let Some(launch) = gst_launch(settings) else {
        return Availability::Missing {
            part: MissingPart::Runtime,
            reason: "gst-launch-1.0 was not found in lib, in the environment or on the PATH"
                .to_owned(),
        };
    };
    if let Err(reason) = runtime_fits(&launch) {
        return Availability::Missing { part: MissingPart::Runtime, reason };
    }
    let version = version_of(&launch).unwrap_or_else(|| "GStreamer".to_owned());
    Availability::Ready { version, location: launch }
}

/// Whether the runtime beside the launcher can load a plugin built with MSVC.
///
/// GStreamer on Windows exists in two builds that name their libraries differently: MSVC
/// calls them `gstreamer-1.0-0.dll`, MinGW calls them `libgstreamer-1.0-0.dll`. The plugin
/// imports the MSVC names, so a MinGW runtime cannot load it no matter what is on the search
/// path. That is a reason to offer the install, not a runtime that is ready.
fn runtime_fits(launch: &std::path::Path) -> Result<(), String> {
    let Some(directory) = launch.parent() else {
        return Ok(());
    };
    if directory.join("gstreamer-1.0-0.dll").is_file() {
        return Ok(());
    }
    if directory.join("libgstreamer-1.0-0.dll").is_file() {
        return Err(format!(
            "the GStreamer at {} is a MinGW build; the plugin is built with MSVC and needs the MSVC runtime, which the installer provides",
            directory.display()
        ));
    }
    Ok(())
}

/// Looks for the FFmpeg programs a session needs.
pub fn ffmpeg(settings: &Settings) -> Availability {
    let Some(programs) = ffmpeg_programs(settings) else {
        return Availability::Missing {
            part: MissingPart::Runtime,
            reason: "ffmpeg and ffplay were not found in lib, in the environment or on the PATH"
                .to_owned(),
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
/// The decoder is looked for where it was named, then under `lib` beside the executable, then
/// in the environment, then on the PATH. The player is looked for beside the decoder first,
/// because a distribution keeps them together: a machine may have ffmpeg from a package
/// manager and ffplay from nowhere at all.
pub fn ffmpeg_programs(settings: &Settings) -> Option<FfmpegPrograms> {
    let decoder = named_program(&settings.backend.ffmpeg.decoder)
        .or_else(|| find_in("ffmpeg", &bundled_directories()))
        .or_else(|| env_program("AIRPLAY_FFMPEG"))
        .or_else(|| find_in("ffmpeg", &environment_directories("PARLOR_FFMPEG_DIR")))
        .or_else(|| find_program("ffmpeg", &[]))?;
    let mut beside = Vec::new();
    if let Some(parent) = decoder.parent() {
        beside.push(parent.to_path_buf());
    }
    beside.extend(bundled_directories());
    beside.extend(environment_directories("PARLOR_FFMPEG_DIR"));
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

/// Finds gst-launch-1.0, which the GStreamer backend runs for video.
pub fn gst_launch(settings: &Settings) -> Option<PathBuf> {
    named_program(&settings.backend.gstreamer.launch_path)
        .or_else(|| find_in("gst-launch-1.0", &bundled_directories()))
        .or_else(|| env_program("AIRPLAY_GST_LAUNCH"))
        .or_else(|| find_in("gst-launch-1.0", &environment_directories("PARLOR_GSTREAMER_DIR")))
        .or_else(|| find_program("gst-launch-1.0", &[]))
}

/// Whether the GStreamer runtime has an element installed.
///
/// The inspector beside the launcher is the runtime's own answer; it needs no GStreamer
/// headers and costs one short process. An absent inspector means "not known", which is
/// answered false so the caller keeps the software default rather than guessing.
pub fn has_element(launch: &std::path::Path, element: &str) -> bool {
    let name = if cfg!(windows) { "gst-inspect-1.0.exe" } else { "gst-inspect-1.0" };
    let inspector = launch.with_file_name(name);
    if !inspector.is_file() {
        return false;
    }
    let mut command = std::process::Command::new(inspector);
    command.arg(element).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
    #[cfg(windows)]
    {
        // The inspector is a console program; starting it from the window must not flash a
        // console of its own.
        use std::os::windows::process::CommandExt as _;
        command.creation_flags(0x0800_0000);
    }
    command.status().map(|status| status.success()).unwrap_or(false)
}

/// The directories beside the application that a bundled backend can live in.
///
/// These are the first place a program is looked for, so a portable distribution is decided
/// by what is in its own directory rather than by what the machine happens to have.
pub fn bundled_directories() -> Vec<PathBuf> {
    let Some(executable) = executable_directory() else {
        return Vec::new();
    };
    BUNDLED_LAYOUT.iter().map(|layout| executable.join(layout)).collect()
}

/// The directories a root environment variable names, as a runtime and as a plain directory.
pub fn environment_directories(variable: &str) -> Vec<PathBuf> {
    // The process copy is what a normal run has. A run started from a shell older than the
    // installation has the old copy, so the registry is asked when the process says nothing.
    let named = std::env::var_os(variable)
        .map(PathBuf::from)
        .or_else(|| super::install::recorded(variable));
    let Some(root) = named else {
        return Vec::new();
    };
    vec![root.join("bin"), root]
}

/// The directories a plugin's own dependencies can be loaded from.
///
/// The loader adds these before opening the plugin: a bundled runtime keeps its DLLs in
/// `lib`, and a runtime named by the environment keeps them in its `bin`.
pub fn plugin_directories() -> Vec<PathBuf> {
    let mut directories = bundled_directories();
    directories.extend(environment_directories("PARLOR_GSTREAMER_DIR"));
    directories
}

/// The location the GStreamer plugin is expected at.
///
/// The plugin is part of the application, so it is looked for beside the executable and then
/// in `lib`. A build that could not produce it is reported by the caller.
pub fn plugin_path() -> Option<PathBuf> {
    let executable = std::env::current_exe().ok()?;
    let directory = executable.parent()?;
    let name = parlor_backend_api::plugin_file_name();
    let beside = directory.join(name);
    if beside.is_file() {
        return Some(beside);
    }
    Some(directory.join("lib").join(name))
}

/// The directory the executable is in.
fn executable_directory() -> Option<PathBuf> {
    let executable = std::env::current_exe().ok()?;
    executable.parent().map(Path::to_path_buf)
}

/// A program the caller named, which is a path or a name on the PATH.
fn named_program(named: &str) -> Option<PathBuf> {
    let named = named.trim();
    if named.is_empty() {
        return None;
    }
    find_program(named, &[])
}

/// A program an environment variable names.
fn env_program(variable: &str) -> Option<PathBuf> {
    let named = std::env::var_os(variable)?;
    if named.is_empty() {
        return None;
    }
    find_program(&named.to_string_lossy(), &[])
}

/// Finds a program in these directories only.
///
/// A name with a separator in it is a path and is used as one. On Windows each candidate is
/// also tried with an executable extension, because a name is not a file name.
pub fn find_in(named: &str, directories: &[PathBuf]) -> Option<PathBuf> {
    let named = named.trim();
    if named.is_empty() {
        return None;
    }
    if named.contains(['/', '\\']) {
        let path = PathBuf::from(named);
        return path.is_file().then_some(path);
    }
    directories.iter().find_map(|directory| candidate(directory, named))
}

/// Finds a program in these directories, and then on the PATH.
pub fn find_program(named: &str, directories: &[PathBuf]) -> Option<PathBuf> {
    find_in(named, directories).or_else(|| find_in(named, &path_directories()))
}

/// Every directory on the PATH.
fn path_directories() -> Vec<PathBuf> {
    std::env::var_os("PATH").map(|path| std::env::split_paths(&path).collect()).unwrap_or_default()
}

/// One candidate file name in one directory.
fn candidate(directory: &Path, named: &str) -> Option<PathBuf> {
    let direct = directory.join(named);
    if direct.is_file() {
        return Some(direct);
    }
    if cfg!(windows) && !named.to_ascii_lowercase().ends_with(".exe") {
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
    use super::{BUNDLED_LAYOUT, candidate, find_in, find_program};

    #[test]
    fn an_empty_name_is_not_a_program() {
        assert_eq!(find_program("", &[]), None);
    }

    #[test]
    fn a_dotted_name_still_finds_its_executable() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        std::fs::write(directory.path().join("gst-launch-1.0.exe"), b"").expect("a file");
        let found = find_in("gst-launch-1.0", &[directory.path().to_path_buf()]);
        assert!(found.is_some(), "a name with a dot is still a name");
    }

    #[test]
    fn a_program_is_found_in_a_directory() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let program = directory.path().join(if cfg!(windows) { "probe.exe" } else { "probe" });
        std::fs::write(&program, b"").expect("a file");
        let found = find_program("probe", &[directory.path().to_path_buf()]);
        assert_eq!(found.as_deref(), Some(program.as_path()));
        assert!(candidate(std::path::Path::new("."), "nothing-here").is_none());
    }

    #[test]
    fn the_bundled_layout_is_lib_beside_the_executable() {
        assert_eq!(BUNDLED_LAYOUT, ["lib", "lib/bin", "lib/gstreamer/bin"]);
    }

    #[test]
    fn a_bundled_directory_is_preferred_to_the_next_one() {
        let bundled = tempfile::tempdir().expect("a temporary directory");
        let environment = tempfile::tempdir().expect("a temporary directory");
        let name = if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" };
        std::fs::write(bundled.path().join(name), b"bundled").expect("a file");
        std::fs::write(environment.path().join(name), b"environment").expect("a file");
        let found =
            find_in("ffmpeg", &[bundled.path().to_path_buf(), environment.path().to_path_buf()])
                .expect("a program");
        assert_eq!(
            std::fs::read(&found).expect("the file"),
            b"bundled",
            "the bundled copy comes first"
        );
    }

    #[test]
    fn an_msvc_runtime_is_accepted_and_a_mingw_one_is_not() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let launch = directory.path().join("gst-launch-1.0.exe");
        std::fs::write(&launch, b"").expect("a launcher");
        assert!(
            super::runtime_fits(&launch).is_ok(),
            "no DLLs beside the launcher: not proven either way"
        );

        std::fs::write(directory.path().join("libgstreamer-1.0-0.dll"), b"").expect("a MinGW DLL");
        let reason = super::runtime_fits(&launch).expect_err("a MinGW runtime is not usable");
        assert!(reason.contains("MinGW"), "{reason}");

        std::fs::write(directory.path().join("gstreamer-1.0-0.dll"), b"").expect("an MSVC DLL");
        assert!(super::runtime_fits(&launch).is_ok(), "the MSVC runtime beside the launcher");
    }
}
