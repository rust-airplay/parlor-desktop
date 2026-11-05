# Parlor

[![CI](https://github.com/rust-airplay/parlor-desktop/actions/workflows/ci.yml/badge.svg)](https://github.com/rust-airplay/parlor-desktop/actions/workflows/ci.yml)
[![Release](https://github.com/rust-airplay/parlor-desktop/actions/workflows/release.yml/badge.svg)](https://github.com/rust-airplay/parlor-desktop/actions/workflows/release.yml)

<img src="assets/icon-128.png" width="80" align="right" alt="Parlor">

Parlor is an AirPlay receiver with a desktop interface for Windows. A device sees Parlor in
its AirPlay menu, pairs with a PIN, and mirrors its screen or plays its audio.

It is the window around the receiver: it keeps the settings a person chooses, starts and stops
the receiver, shows the PIN and what has connected, and finds - or installs - the programs
that decode and present the streams. The protocol and the media handling are not here.
[`rust-airplay`](https://github.com/rust-airplay/rust-airplay) answers the device and
decrypts the streams; [`rust-airplay-gstream`](https://github.com/rust-airplay/rust-airplay-gstream)
and [`rust-airplay-ffmpeg`](https://github.com/rust-airplay/rust-airplay-ffmpeg) play them.

## Features

- **Screen mirroring and audio** from AirPlay clients, with a one-time or fixed PIN.
- **Two playback backends, switchable in the application:**
  - **GStreamer**, through a plugin loaded at run time. Hardware decoders are used when the
    runtime has them.
  - **FFmpeg**, through `ffmpeg` and `ffplay`, with no native build dependency.
- **One-click install** of a missing backend into `%LOCALAPPDATA%\Parlor\backends`, with the
  environment recorded so the next start finds it.
- **The whole receiver configuration**: identity, name, ports, bind address, announcement,
  pairing, password, client policy, remembered devices, the advertised display and frame
  rate, the feature bitmap, audio codec and latency, the volume curve, video pacing, and
  the diagnostic files.
- **A home page for the controls reached while watching**: start and stop, session name,
  mirror size, frame rate and backend.
- **Light and dark themes**, following the system by default, and **English and Simplified
  Chinese**.
- **Update check** in About: the latest release names itself in [`Version`](Version), and
  the button opens the release page when a newer one exists.
- **One executable to install**: the GStreamer adapter is embedded in the program and
  unpacked under the data directory on first use.

The interface uses no emoji.

## Requirements

- Windows 10 or later, 64 bit.
- Rust 1.95 or newer with the MSVC toolchain, to build.
- No media library is needed to build. GStreamer and FFmpeg are found at run time, and the
  application can install either one when a machine has neither.

## Building

```powershell
# The debug build, and the GStreamer plugin beside it.
scripts\build.ps1

# The release build.
scripts\build.ps1 -Release
```

`scripts\run.ps1` starts the debug build. The workspace has three crates:
`parlor-desktop` is the program, `parlor-backend-api` is the plugin boundary, and
`parlor-gst-backend` is the GStreamer plugin.

## Running

Start `parlor-desktop.exe`. On the first run the home page offers to install a playback
backend when the machine has none; choose GStreamer when it can be installed, because it
decodes on the GPU.

The device connects over the local network. Pair with the PIN the application shows, then
mirror the screen or play audio from the AirPlay menu.

## Playback backends

| | GStreamer | FFmpeg |
| --- | --- | --- |
| Delivery | Embedded in the program, unpacked on first use; loads GStreamer at run time | `ffplay` / `ffmpeg` programs |
| Decoding | GPU (`d3d11h264dec`, `d3d11h265dec`) when available | CPU, or GPU with hardware decoding on |
| Latency | Lowest | Higher; it is the fallback |
| Install | One click | One click |

The FFmpeg video window defaults to 1280x720 because `ffplay` sizes its window to the
video. Choose 1600x900 or fullscreen in **Settings > Playback > FFmpeg > Video output**.

## Settings worth knowing

- **Mirror size.** A mirror is the device's own picture scaled to the advertised size, so
  1920x1080 stays sharp; larger sizes ask the device to upscale. The FFmpeg backend is capped
  at 1080p while it decodes in software.
- **Hardware decoding** is on by default. The FFmpeg renderer asks its decoder for the
  accelerator the build has and falls back to software when there is none.
- **Settings apply when the receiver starts.** A changed setting shows a restart notice on
  the home page and in the rail.
- Files live in `%LOCALAPPDATA%\Parlor`: `settings.toml`, the identity, `logs\parlor.log`
  and the installed backends.

## Updating

The latest release names its version in [`Version`](Version). **Settings > About > Check for
updates** reads that file and, when it names a newer version, offers the release page.

A new version is installed by replacing the old program, so the archive contains
`parlor-desktop.exe`, this README, the licence and the version file. Settings are kept in
`%LOCALAPPDATA%\Parlor` and survive the replacement.

## Testing

```powershell
cargo test -p parlor-desktop -p parlor-backend-api
cargo clippy -p parlor-desktop -p parlor-backend-api --all-targets -- -D warnings
cargo fmt --all -- --check
```

The two renderer tests are ignored by default because they need a real backend:

```powershell
cargo build -p parlor-gst-backend
copy target\debug\parlor_gst_backend.dll target\debug\deps\
$env:PARLOR_GSTREAMER_DIR = "$PWD\.tools\gstreamer\runtime"
cargo test -p parlor-desktop --bin parlor-desktop -- --ignored --test-threads=1
```

## Releasing

Run **Actions > Release > Run workflow** with the tag to create, for example `v0.2.0`, and
optionally the notes and a draft flag. The workflow checks that the tag matches
`Cargo.toml`, builds and tests on `windows-latest`, packages the program and plugin, and
creates the GitHub release. It then writes the version into `Version` on `main`, which is
what the update check reads.

## Documentation

- [`docs/BUILDING.md`](docs/BUILDING.md) - the toolchains, the icon, and the GStreamer
  loader.
- [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) - how the window, the session and the
  backends fit together.
- [`docs/feature-inventory.md`](docs/feature-inventory.md) - every receiver feature and
  where it is configured.
- [`AGENT.md`](AGENT.md) - the brief this application was built to.

## License

GPL-3.0-or-later. See [`LICENSE`](LICENSE).
