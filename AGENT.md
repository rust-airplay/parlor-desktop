# Working on this repository

This file describes how to build, test and change the code, for an automated contributor.

## Layout

Three crates, one of which exists only because of a platform constraint.

```
crates/parlor-desktop/       the application
  src/app.rs                 bootstrap: paths, logger, globals, window
  src/settings/              the settings document, storage, the identity seed and clients
  src/session/               one receiver on one thread, its configuration and its events
  src/render/                backend choice, detection, the plugin loader, installation
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
```

The plugin is built by `scripts/build.ps1` when `scripts/setup-gstreamer.ps1` has run. Its
gate is the same `clippy` with `-p parlor-gst-backend`, on a machine with the development
files.

## Rules

The application must start on a machine with no playback backend: that is why the
GStreamer backend is a plugin and why nothing in `parlor-desktop` links a native library.
A change that links one breaks the first-run installation path.

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