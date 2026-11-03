# Parlor

An AirPlay receiver for the desktop, built on the [rust-airplay](https://github.com/rust-airplay)
libraries.

A device sees Parlor in its AirPlay menu, pairs with a PIN, and mirrors its screen or plays
its audio. Parlor is the program around the receiver: it keeps the settings a person chooses,
starts and stops the receiver, shows the PIN and what has connected, and looks for what a
machine has to play through before it starts.

The protocol and the media handling are not here. `rust-airplay` answers the device and
decrypts the streams; a playback backend decodes and presents them. Parlor owns the window,
the session's lifetime and both backends' requirements.

## What it does

- Receives screen mirroring and audio from AirPlay clients, with the one-time or fixed PIN
  the protocol asks for.
- Plays through **FFmpeg** (two programs, no native build dependency) or **GStreamer**
  (a plugin loaded at run time), switched in the settings.
- Finds a backend in `lib` beside the application first, then through the environment
  (`PARLOR_GSTREAMER_DIR`, `PARLOR_FFMPEG_DIR`, the backends' own `AIRPLAY_*` variables, and
  finally the `PATH`). Nothing is downloaded or installed by the program.
- Covers the receiver's whole configuration surface: identity, name, ports, bind address,
  announcement, pairing, password, client policy, remembered devices, the advertised
  display and frame rate, the feature bitmap, audio codec and latency, the volume curve,
  the video pacing decision, and every diagnostic file.
- Puts the controls a person reaches for while watching on the home page: start and stop,
  resolution, frame rate, and the backend.
- Has a light and a dark theme, follows the operating system's appearance, and speaks
  English and Simplified Chinese.

The interface uses no emoji.

## Requirements

- Windows 10 or later, 64 bit.
- Rust 1.92 or newer with the MSVC toolchain.
- Visual Studio 2022 Build Tools with the *Desktop development with C++* workload, which is
  what GPUI compiles against.
- CMake on `PATH` for the same reason.
- For the GStreamer backend only: the GStreamer MSVC development files, which
  `scripts/setup-gstreamer.ps1` fetches, and `pkg-config` on `PATH` (for example the
  one MSYS2 or `pkg-config-lite` provides).

The FFmpeg backend needs neither at build time.

## Building

```powershell
# Optional: the GStreamer runtime and development packages, unpacked under .tools
powershell -ExecutionPolicy Bypass -File scripts\setup-gstreamer.ps1

# The application, and the GStreamer plugin when the development files are present
powershell -ExecutionPolicy Bypass -File scripts\build.ps1 -Release
```

Without the GStreamer development files the application still builds, and the GStreamer
backend reports that its plugin is missing. That is deliberate: the application must start
on a machine that has no GStreamer so that it can say where one goes.

The plugin is a separate library, `parlor_gst_backend.dll`, built beside the executable.
The application loads it only when the GStreamer backend is chosen.

## Running

```powershell
powershell -ExecutionPolicy Bypass -File scripts\run.ps1
```

Then start the receiver from the home page and choose Parlor in the AirPlay menu of a
device. If the receiver is running and no device has paired with this machine before, the
device asks for the PIN the window shows.

A release build runs from `target\release\parlor-desktop.exe`. The plugin beside it is
loaded on demand; the GStreamer runtime is looked for in the application's own data
directory, beside the executable, and in `PARLOR_GSTREAMER_DIR`.

## Backends

| | FFmpeg | GStreamer |
| --- | --- | --- |
| Build dependency | none | GStreamer MSVC development files |
| Run dependency | `ffmpeg.exe` and `ffplay.exe` | the GStreamer runtime, its plugins, and `gst-launch-1.0` |
| Loaded | in the application | from `parlor_gst_backend.dll` |
| Where it is found | `lib` beside the executable, then `PARLOR_FFMPEG_DIR`, then `PATH` | `lib` beside the executable, then `PARLOR_GSTREAMER_DIR`, then `PATH` |

Both backends play video in their own window: a backend receives coded frames and presents
them through its own pipeline, and a frame never enters the window's compositor. Parlor is
the receiver's interface, not a video widget.

## Settings and files

Settings live in `%LOCALAPPDATA%\Parlor\settings.toml`, written on every change, and are
a plain TOML document a person may read and edit while Parlor is closed. The same directory
holds the identity seed, the remembered clients and the log.

Every value of the receiver's configuration has a page in the application;
[docs/feature-inventory.md](docs/feature-inventory.md) maps each one to the page that shows
it and states what it does.

## Project layout

```
crates/parlor-desktop/       the application: window, settings, session, backends, detection
  src/app.rs                 bootstrap: files, logger, window
  src/settings/              the settings document, its storage and the identity
  src/render/                backend choice, detection and the plugin loader
  src/session/               a receiver on its own thread, and its events
  src/ui/                    the window: rail, home page, settings pages
  src/i18n/                  every label, in English and Chinese
crates/parlor-backend-api/   the contract a renderer plugin implements
crates/parlor-gst-backend/   the GStreamer plugin
docs/                        architecture, building, and the receiver's feature inventory
scripts/                     setup, build and run
```

## Testing

```powershell
cargo test -p parlor-desktop -p parlor-backend-api
```

Two tests build a real renderer and are ignored by default because they need what only a
machine with a backend has:

```powershell
# with the plugin beside the test binary and a GStreamer runtime to load
cargo test -p parlor-desktop -- --ignored
```

## Licence

GNU General Public License, version 3 or later. The receiver libraries are GPL-3.0-or-later
and a program that links them is bound by the same licence.

中文说明: [README.zh-CN.md](README.zh-CN.md)