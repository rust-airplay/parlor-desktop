# Building

## Prerequisites

| Requirement | Why | Where |
| --- | --- | --- |
| Windows 10 or later, 64 bit | the application targets Windows | -- |
| Rust 1.92+ with the MSVC toolchain | the workspace is edition 2024 | https://rustup.rs |
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
on a machine with a MinGW GStreamer on the `PATH`: its libraries are named
`libgstreamer-1.0-0.dll`, which an MSVC plugin cannot import, so the application reports that
runtime as missing instead of listing it as ready.

## Crashes in development builds

Development builds are compiled without debug assertions. GPUI asserts, only when debug
assertions are present, that an accessibility frame does not name two focused nodes; a video
window that takes focus and gives it back reaches that path, and the assertion aborts the
process instead of warning as it does in release. Overflow checks stay on.

A panic is written to `logs/parlor.log` even when the log level is Off, with a backtrace, so
a window that disappears leaves the reason behind.

## The icon

`assets/icon.ico` is embedded in the executable by `crates/parlor-desktop/build.rs`, which is
what puts it in Explorer, the taskbar and the window title bar. The files are generated from
one description:

```powershell
python scripts\draw-icon.py
```

That writes `assets/icon.png` (1024), the sizes from 16 to 256, and `assets/icon.ico`. The
drawing is done at four times the final size and downsampled, which is what keeps the small
sizes clean.

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
cargo test -p parlor-desktop --bin parlor-desktop -- --ignored --test-threads=1
```

The window tests (`ui::tests`) need neither a display nor a backend: they open a headless
GPUI window, click the controls the way a pointer would and assert on the settings document
that the click wrote. They come with the normal `cargo test` run.

The ignored tests are serialized because several of them use the process environment, which
is shared: one test sets `PARLOR_FFMPEG_DIR` for the length of its run, and a session test
started at the same moment would discover that directory instead of the machine's own
programs. Two of them download a backend package, so the suite needs a network and a few
minutes.

## Packing an application

`target\release\parlor-desktop.exe` is the application, and `parlor_gst_backend.dll` belongs
beside it. A distribution can carry either backend in a `lib` directory beside the executable:

```text
parlor-desktop.exe
parlor_gst_backend.dll
lib\gst-launch-1.0.exe, lib\*.dll           flat runtime copy
lib\bin\gst-launch-1.0.exe, lib\bin\*.dll   or the runtime's own bin directory
lib\gstreamer\bin\...                        or a runtime unpacked under its own name
lib\ffmpeg.exe, lib\ffplay.exe               FFmpeg programs
```

The application looks in `lib` first, then in `PARLOR_GSTREAMER_DIR` or `PARLOR_FFMPEG_DIR`,
then on the `PATH`. Nothing is required for the application to start: a machine with neither
backend shows the home page's "no playback backend" note, which offers to install one and says
where a hand-placed one goes.

An installation from the application goes to `%LOCALAPPDATA%\\Parlor\\backends\\<backend>`
and writes that directory into the same `PARLOR_*` variable, for the current user. Nothing is
installed machine-wide and no administrator prompt is involved.

## Troubleshooting

**"the GStreamer plugin is missing"** — the application did not find
`parlor_gst_backend.dll` beside the executable. Build it with
`cargo build -p parlor-gst-backend` after running `setup-gstreamer.ps1`.

**The plugin will not load** — the GStreamer runtime is absent, is not the MSVC build, or is
not reachable. The application adds the directory of the `gst-launch-1.0` it resolved to the
DLL search path before it opens the plugin, so a runtime named in Settings or by
`PARLOR_GSTREAMER_DIR` needs no `PATH` entry. Install it from Settings → Playback, copy a
runtime into `lib` beside the executable, or set `PARLOR_GSTREAMER_DIR` to the runtime
directory. A program launched from a shell that predates the installation inherits that
shell's old environment; detection falls back to the registry copy the installer wrote, so the
runtime is found either way. A MinGW runtime from MSYS2 cannot be used: its libraries are named
`libgstreamer-1.0-0.dll` while the plugin imports `gstreamer-1.0-0.dll`, and detection now
reports that as a missing runtime so the installer can supply the MSVC one.

**A debug build stops with "set_focus called more than once in a single frame"** — that is a
GPUI assertion compiled in only when debug assertions are on. It fires while a video window
takes and returns focus, and release builds log a warning and carry on. Use a release build
(`scripts\build.ps1 -Release`) for anything that opens a session.

**The picture is late, or shows damaged frames** — software decoding can fall behind a desktop
mirror, and a pipe that cannot keep up grows its latency and starts losing frames. When the
GStreamer runtime has `d3d11h264dec` or `d3d11h265dec`, the application fills an empty decoder
field with it and says so in the log; type `avdec_h264` or `avdec_h265` in the decoder field to
go back to software. AirPlay mirroring rarely needs more than 1920x1080 at 60 frames, and a
larger or faster display target only makes the sender encode more than the link can carry.

**GStreamer plugins fail to load ("找不到指定的模块")** — GStreamer opens its plugins with the
altered search path, which reads `PATH` and not the directories the application registers for
the process. The application puts the runtime's `bin` on `PATH` before it builds a GStreamer
renderer, which is what lets `gstlibav` find `avcodec`; a runtime whose DLLs are elsewhere
(such as the MSYS2 build) still cannot be used.

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