# Building

## Prerequisites

| Requirement | Why | Where |
| --- | --- | --- |
| Windows 10 or later, 64 bit | the application targets Windows | -- |
| Rust 1.90+ with the MSVC toolchain | the workspace is edition 2024 | https://rustup.rs |
| Visual Studio 2022 Build Tools, *Desktop development with C++* | GPUI compiles C++-adjacent parts and links the Windows SDK | https://visualstudio.microsoft.com/visual-cpp-build-tools/ |
| CMake on `PATH` | GPUI's build scripts | https://cmake.org |
| GStreamer MSVC development files, for the plugin | `gstreamer-sys` links the library | `scripts/setup-gstreamer.ps1` |
| `pkg-config` on `PATH`, for the plugin | how the GStreamer crates find their libraries | MSYS2, or `pkg-config-lite` |

The FFmpeg backend needs no native development files: it runs `ffmpeg.exe` and `ffplay.exe`,
which the application can install itself.

## Steps

```powershell
git clone https://github.com/rust-airplay/parlor-desktop
cd parlor-desktop
powershell -ExecutionPolicy Bypass -File scripts\setup-gstreamer.ps1
powershell -ExecutionPolicy Bypass -File scripts\build.ps1 -Release
```

`setup-gstreamer.ps1` downloads the official MSVC runtime and development packages for
GStreamer 1.26.11 and unpacks them under `.tools\gstreamer`. An administrative install is
used, so nothing is installed into the machine and no administrator rights are needed.

`build.ps1` sets `PKG_CONFIG_PATH` to the unpacked development package, builds the
application, and then builds the plugin. Without the development files it builds the
application alone and says so.

## Running a development build

```powershell
powershell -ExecutionPolicy Bypass -File scripts\run.ps1
```

`run.ps1` sets `PARLOR_GSTREAMER_DIR` to the unpacked runtime, so a development run uses the
runtime the plugin was built against rather than whatever else is on the machine. That matters
on a machine with a MinGW GStreamer on the `PATH`: its DLLs have the same names and a
different build.

## Tests

```powershell
cargo test -p parlor-desktop -p parlor-backend-api
cargo clippy -p parlor-desktop -p parlor-backend-api --all-targets -- -D warnings
cargo fmt --all -- --check
```

The two renderer tests are ignored by default. They need a real backend:

```powershell
cargo build -p parlor-gst-backend
copy target\debug\parlor_gst_backend.dll target\debug\deps\
$env:PARLOR_GSTREAMER_DIR = "$PWD\.tools\gstreamer\runtime"
cargo test -p parlor-desktop -- --ignored
```

## Packing an application

`target\release\parlor-desktop.exe` is the application. Beside it belong:

- `parlor_gst_backend.dll`, when the build included the plugin;
- optionally a GStreamer runtime under `gstreamer\bin`, which the application prefers over a
  system one;
- optionally `ffmpeg.exe` and `ffplay.exe` under `backends\ffmpeg\bin`, which the FFmpeg
  backend finds without a download.

None of these are required for the application to start. A machine with nothing installed
shows the home page's "no playback backend" offer, and can install either backend from there.

## Troubleshooting

**"the GStreamer plugin is missing"** — the application did not find
`parlor_gst_backend.dll` beside the executable. Build it with
`cargo build -p parlor-gst-backend` after running `setup-gstreamer.ps1`.

**The plugin will not load** — the GStreamer runtime is absent or of another build. Set
`PARLOR_GSTREAMER_DIR` to the runtime directory, or use the settings page to install it. A
MinGW runtime from MSYS2 is not the runtime an MSVC plugin links against.

**`pkg-config` cannot find `gstreamer-1.0`** — `PKG_CONFIG_PATH` does not point at the
unpacked development package. `build.ps1` sets it; a manual `cargo build` needs it set by
hand.

**The port is in use** — the receiver binds 7000 by default. Change the control port on the
Network page, or stop the other receiver.

**A device does not see the receiver** — the announcement is on by default. A firewall that
blocks mDNS or the control port is the usual cause; the receiver's ports are on the Network
page.

**The PIN never appears** — a device only asks for one while it pairs. The home page shows the
PIN when the receiver reports it.
