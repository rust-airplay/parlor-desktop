# Working on this repository

This file describes how to build, test and change the code, for an automated contributor.

## Layout

Three crates, one of which exists only because of a platform constraint.

```
crates/parlor-desktop/       the application
  src/app.rs                 bootstrap: paths, logger, globals, window
  src/settings/              the settings document, storage, the identity seed and clients
  src/session/               one receiver on one thread, its configuration and its events
  src/render/                backend choice, detection and the plugin loader
  src/ui/                    the rail, the home page, the settings pages
  src/i18n/                  every label, English and Simplified Chinese
crates/parlor-backend-api/   the ABI a renderer plugin implements
crates/parlor-gst-backend/   the GStreamer plugin; the only crate that links GStreamer
docs/                        architecture, building, feature inventory, API notes
scripts/                     setup, build, run
```

## Gates

A change is finished when all of these pass:

```powershell
cargo fmt --all -- --check
cargo clippy -p parlor-desktop -p parlor-backend-api --all-targets -- -D warnings
cargo test -p parlor-desktop -p parlor-backend-api
cargo test -p parlor-desktop --bin parlor-desktop -- --ignored --test-threads=1
```

The last line runs the tests that need what only a machine has: a playback backend, a
network for the two installer downloads, and a free local port for the session tests. They
are serialized because they share the process environment. `--test-threads=1` is not
optional: one test sets `PARLOR_FFMPEG_DIR`, and a concurrent session test would pick it up.

The first command also runs the window's own tests (`ui::tests`): they open a headless
window, click the real controls and read the document those clicks wrote. A dropdown must
hand its writer the value the setting is stored as, not the label the user sees, and the
home quick choices must reach both the in-memory document and the file on disk. No display
and no backend is needed.

The plugin is built by `scripts/build.ps1` when `scripts/setup-gstreamer.ps1` has run. Its
gate is the same `clippy` with `-p parlor-gst-backend`, on a machine with the development
files.

## Rules

The application must start on a machine with no playback backend: that is why the
GStreamer backend is a plugin and why nothing in `parlor-desktop` links a native library.
A change that links one breaks the first run, where the window has to say where a backend
goes.

Backends are found in one order: the explicit setting, `lib` beside the executable, the
backends' `AIRPLAY_*` variables, `PARLOR_GSTREAMER_DIR`/`PARLOR_FFMPEG_DIR`, and the `PATH`
last.

An installation under the data directory is found through the environment, not through a
branch in detection: the installer writes `PARLOR_GSTREAMER_DIR` or `PARLOR_FFMPEG_DIR` for
the current user (HKCU) and sets it in this process, and a removal clears it. Adding the
data directory to detection instead would make two sources of truth and hide a variable
the user set by hand.

The receiver reads its configuration once. A setting that needs to be live is a change to
the receiver, not a change to this program; say so in the message.

`unsafe` is denied by the workspace lints. The plugin loader and the locale probe are the
two places that allow it, each with the reason beside it.

Comments explain why. The code says what it does.

The style is the Google Rust style guide, with two exceptions the ecosystem forces: the
UI modules import GPUI with `use gpui_kit::*;` because that is the kit's documented usage and the Styled and AppContext traits are reached through it, and the program is `async` where the receiver and GPUI are. An import list that names every element helper would be the one thing on the page harder to read than the page.

No emoji, in code, logs or documents.

Documentation is English; the README also exists in Chinese. A guide under `docs/` states
what was run and what was only read, and a platform claim names the platform.