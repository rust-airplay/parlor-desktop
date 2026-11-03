// SPDX-License-Identifier: GPL-3.0-or-later
//! Where the application keeps what it has to remember.
//!
//! Everything lives under one directory, so a person can find it, copy it to another
//! machine, or delete it to start over. The directory is the operating system's per-user
//! data directory, or the one PARLOR_DATA_DIR names.

use std::path::{Path, PathBuf};

use super::Settings;

/// Where the application keeps its files.
#[derive(Clone, Debug)]
pub struct Paths {
    root: PathBuf,
}

impl Paths {
    /// The environment variable that overrides the data directory.
    ///
    /// A test, a portable installation and a person with two receivers all want this.
    pub const DATA_DIR_ENV: &'static str = "PARLOR_DATA_DIR";

    /// The settings document.
    pub const SETTINGS_FILE: &'static str = "settings.toml";
    /// The identity seed, as hexadecimal.
    pub const SEED_FILE: &'static str = "identity.seed";
    /// The clients a previous session verified, as hexadecimal keys in JSON.
    pub const CLIENTS_FILE: &'static str = "clients.json";
    /// The directory a run's log files are written to.
    pub const LOGS_DIR: &'static str = "logs";

    /// The directory this user's installation keeps its files in.
    ///
    /// # Errors
    ///
    /// Returns [Error::NoDataDirectory] when the environment names none and the platform
    /// has none either, which is a machine that cannot store a setting at all.
    pub fn discover() -> Result<Self, Error> {
        if let Some(directory) = std::env::var_os(Self::DATA_DIR_ENV) {
            return Ok(Self::at(directory));
        }
        let base = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("XDG_DATA_HOME").map(PathBuf::from))
            .or_else(|| std::env::var_os("HOME").map(|home| Path::new(&home).join(".local/share")))
            .ok_or(Error::NoDataDirectory)?;
        Ok(Self::at(base.join("Parlor")))
    }

    /// A directory named by the caller, which is what a test uses.
    pub fn at(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// One entry directly inside the directory.
    pub fn join(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }

    /// The settings document.
    pub fn settings(&self) -> PathBuf {
        self.join(Self::SETTINGS_FILE)
    }

    /// The identity seed.
    pub fn seed(&self) -> PathBuf {
        self.join(Self::SEED_FILE)
    }

    /// The remembered clients.
    pub fn clients(&self) -> PathBuf {
        self.join(Self::CLIENTS_FILE)
    }

    /// The directory a run's log files are written to.
    pub fn logs(&self) -> PathBuf {
        self.join(Self::LOGS_DIR)
    }

    /// Where the request trace is appended.
    pub fn trace_log(&self) -> PathBuf {
        self.logs().join("trace.log")
    }

    /// Where the first video frames are written.
    pub fn video_dump(&self) -> PathBuf {
        self.logs().join("video.h264")
    }

    /// Where the audio payloads are written.
    pub fn audio_dump(&self) -> PathBuf {
        self.logs().join("audio.bin")
    }
}

/// Reads the settings document, or the defaults when there is none yet.
///
/// A document from a newer build is read with what this build understands and reported as
/// such, because refusing to start is a worse answer than losing a field a person never
/// set.
///
/// # Errors
///
/// Returns [Error::Io] when the file exists and cannot be read, and [Error::Parse] when it
/// exists and is not a settings document.
pub fn load(paths: &Paths) -> Result<Settings, Error> {
    let path = paths.settings();
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Settings::new()),
        Err(source) => return Err(Error::Io { path, source }),
    };
    let mut settings: Settings =
        toml::from_str(&text).map_err(|source| Error::Parse { path: path.clone(), source })?;
    if settings.schema_version == 0 {
        settings.schema_version = Settings::SCHEMA_VERSION;
    }
    Ok(settings)
}

/// Writes the settings document, replacing the previous one in a single step.
///
/// The temporary file is what makes the replacement atomic: a crash while writing leaves
/// the previous document, not half of a new one.
///
/// # Errors
///
/// Returns [Error::Io] when the directory or the file cannot be written, and
/// [Error::Serialize] when a value has no TOML form.
pub fn save(paths: &Paths, settings: &Settings) -> Result<(), Error> {
    let path = paths.settings();
    let document = document(settings)?;
    save_text(&path, &document).map_err(|source| Error::Io { path, source })
}

/// The document as text, with a header a person reading it deserves.
fn document(settings: &Settings) -> Result<String, Error> {
    let body = toml::to_string_pretty(settings).map_err(|source| Error::Serialize { source })?;
    Ok(format!(
        "# Parlor settings.\n# Written by the application; edit it while Parlor is closed.\n\n{body}"
    ))
}

/// Writes text to a file through a temporary neighbour.
pub(crate) fn save_text(path: &Path, text: &str) -> std::io::Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent)?;
    let temporary = path.with_extension("tmp");
    std::fs::write(&temporary, text)?;
    std::fs::rename(&temporary, path)?;
    Ok(())
}

/// What can go wrong while the application's files are read or written.
#[derive(Debug)]
pub enum Error {
    /// The platform named no per-user data directory.
    NoDataDirectory,
    /// A file could not be read or written.
    Io {
        /// The file.
        path: PathBuf,
        /// The underlying failure.
        source: std::io::Error,
    },
    /// A document is not the document it should be.
    Parse {
        /// The file.
        path: PathBuf,
        /// The underlying failure.
        source: toml::de::Error,
    },
    /// A value could not be written as TOML.
    Serialize {
        /// The underlying failure.
        source: toml::ser::Error,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoDataDirectory => {
                write!(formatter, "no data directory; name one with PARLOR_DATA_DIR")
            }
            Self::Io { path, source } => write!(formatter, "{}: {source}", path.display()),
            Self::Parse { path, source } => {
                write!(formatter, "{} is not a settings document: {source}", path.display())
            }
            Self::Serialize { source } => write!(formatter, "cannot write the settings: {source}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::NoDataDirectory => None,
            Self::Io { source, .. } => Some(source),
            Self::Parse { source, .. } => Some(source),
            Self::Serialize { source } => Some(source),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(source: std::io::Error) -> Self {
        Self::Io { path: PathBuf::new(), source }
    }
}

#[cfg(test)]
mod tests {
    use super::{Error, Paths, load, save};
    use crate::settings::{LanguageChoice, Settings, ThemeChoice};

    #[test]
    fn a_directory_without_a_document_holds_the_defaults() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let paths = Paths::at(directory.path());
        let settings = load(&paths).expect("the defaults");
        assert_eq!(settings, Settings::new());
    }

    #[test]
    fn a_document_round_trips() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let paths = Paths::at(directory.path());
        let settings = Settings {
            appearance: crate::settings::Appearance {
                theme: ThemeChoice::Dark,
                language: LanguageChoice::Chinese,
            },
            ..Settings::new()
        };
        save(&paths, &settings).expect("a written document");
        assert_eq!(load(&paths).expect("the written document"), settings);
    }

    #[test]
    fn a_missing_field_takes_its_default() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let paths = Paths::at(directory.path());
        std::fs::write(paths.settings(), "schema_version = 1\n[appearance]\ntheme = \"dark\"\n")
            .expect("a document");
        let settings = load(&paths).expect("a partial document");
        assert_eq!(settings.appearance.theme, ThemeChoice::Dark);
        assert_eq!(settings.appearance.language, LanguageChoice::English);
    }

    #[test]
    fn the_backend_names_round_trip() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let paths = Paths::at(directory.path());
        for kind in [crate::settings::BackendKind::GStreamer, crate::settings::BackendKind::Ffmpeg]
        {
            let settings = Settings {
                backend: crate::settings::BackendSettings { kind, ..Default::default() },
                ..Settings::new()
            };
            save(&paths, &settings).expect("a written document");
            let text = std::fs::read_to_string(paths.settings()).expect("the document");
            let expected = match kind {
                crate::settings::BackendKind::GStreamer => "kind = \"gstreamer\"",
                crate::settings::BackendKind::Ffmpeg => "kind = \"ffmpeg\"",
            };
            assert!(text.contains(expected), "{text}");
            assert_eq!(load(&paths).expect("the written document").backend.kind, kind);
        }
    }

    #[test]
    fn an_unreadable_document_is_reported() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let paths = Paths::at(directory.path());
        std::fs::write(paths.settings(), "not = [a, settings, document").expect("a document");
        assert!(matches!(load(&paths), Err(Error::Parse { .. })));
    }
}
