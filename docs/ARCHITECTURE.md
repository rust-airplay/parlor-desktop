# Architecture

Parlor is three layers with one seam each.

| Layer | Crate | Responsibility |
| --- | --- | --- |
| protocol | `rust-airplay` | discovery, pairing, FairPlay, RTSP, media sockets, decryption, RTP, framing |
| playback | `rust-airplay-gstream`, `rust-airplay-ffmpeg` | decode coded frames and present them |
| program | this repository | settings, session lifetime, backends' requirements, window |

Nothing in the program talks to a client. The seam between program and playback is
`rust_airplay::Renderer`: three methods and two streams, all in coded frames.

## Crates

`crates/parlor-desktop` is the application. `crates/parlor-backend-api` is the contract a
renderer plugin implements. `crates/parlor-gst-backend` is the one plugin that exists, and
the only crate that links GStreamer.

```
app.rs            files, logger, globals, window
settings/         the settings document; storage; identity seed and remembered clients
session/          a receiver on a thread, its configuration and its events; the log sink
render/           backend choice; detection; the plugin loader
ui/               rail, home page, settings pages; the session's state as the window sees it
i18n/             every label in English and Simplified Chinese
state.rs          the global the window and the settings pages share
```

## Why the GStreamer backend is a plugin

GStreamer is a native library that is linked, not run. A program that links it does not start
on a machine without it: Windows resolves the import table before `main`, so the failure is a
message box about a missing DLL, not a window that could say where the runtime goes.

The application therefore links nothing native. The GStreamer backend is a `cdylib` built
from the same workspace, and the application opens it when a session asks for that backend.
The ABI is two symbols and one JSON document, described in `parlor-backend-api`; the loader
in `render/gst.rs` adds the runtime's `bin` directory to the DLL search path first, checks
the ABI version, and only then creates a renderer.

The plugin is never unloaded. A renderer keeps code pointers into it for as long as a session
lives, and the process cannot tell when the last of those is gone; keeping one library mapped
costs nothing that matters.

## Session lifetime

A session is a thread, not a future on the window's executor:

1. `Session::start` spawns a thread named `parlor-receiver`.
2. The thread builds a Tokio runtime, then the renderer the settings name. A renderer that
   cannot be built is reported as a failure; the window stays open.
3. The receiver is configured from the settings (see `session/config.rs`), and every library
   callback is a send on one channel: the PIN, a verified client, artwork, metadata, a
   remote-control identifier.
4. `rust_airplay::serve` runs until a stop signal arrives. The window reads the channel on a
   timer, applies what it finds, and repaints.

The receiver reads its configuration once. That is why every settings page says a change
takes effect the next time the receiver starts, and why the window never edits a running
receiver.

## Settings

Settings are a global (`state::Parlor`) rather than a field of the window because the settings
pages read and write them from callbacks that receive only the whole application, and because
the home page shows the same values. Every edit is saved immediately, through a temporary file
and a rename, so a crash cannot leave half a document.

An edit that is invalid is kept but reported: the home page shows the first problem, and
starting a receiver with one is refused. Refusing to save a value a person typed would lose
their work; refusing to start with an unusable one is honest.

## Detection

`render/detect.rs` answers one question per backend: can this machine play through it? The
search is one order, the same for both backends:

```text
explicit setting -> lib beside the executable -> AIRPLAY_* -> PARLOR_* -> PATH
```

`lib` is the first answer because that is what a portable distribution controls: a runtime
copied into `lib` is the runtime this build was made for, in either a flat layout, a `bin`
directory, or a runtime unpacked under its own name. The environment comes second: a file the
program cannot see inside `lib` can be named by `PARLOR_GSTREAMER_DIR` or `PARLOR_FFMPEG_DIR`,
and the backends' own `AIRPLAY_GST_LAUNCH` and `AIRPLAY_FFMPEG` are honoured too.

The `PATH` is last on purpose. A machine with a MinGW GStreamer on its `PATH` has a runtime
whose DLLs have the same names as the MSVC one this plugin was built against, and picking that
one is a crash rather than a session. A program found on the `PATH` is found only after
everything this application controls has been ruled out.

An installation is a download, an unpack and one verification run, on a thread of its own,
under `backends` in the application's data directory. It is recorded the way detection reads:
the installer writes `PARLOR_GSTREAMER_DIR` or `PARLOR_FFMPEG_DIR` for the current user
(HKCU\\Environment, no administrator rights), broadcasts the change, and sets the same value in
the running process so the next session finds it without a restart. A removal clears the
variable when it still names that directory.

That is deliberate: detection has one list of places, and an installation adds itself to a
place on that list instead of adding a second list. A data directory that detection also knew
about would make two sources of truth, and would hide a variable a person set by hand.

When neither backend is found, the window offers to install one and says where a hand-placed
one goes: `lib` beside the executable, or the same environment variable.

## The window

GPUI Kit supplies the components, the theme and the window. The window is one view
(`ui::RootView`) that owns the running session and reads the settings global; the home page
and the settings pages are functions over that view. The settings pages use the kit's
`setting::Settings`, which renders its own list of pages and its own fields.

Text comes from `i18n::Text`: one enum whose variants each carry both languages, so a label
added to one language and not the other does not compile. The theme follows
`appearance.theme`; when that is "system", the window observes the operating system's
appearance and re-applies it.

## What the application does not show

Video. A backend receives coded frames and presents them through its own pipeline, so a
mirror appears in the backend's own window (`ffplay`, or a GStreamer sink). Parlor is the
receiver's interface: it shows what is happening, not the picture. Putting the picture in the
window would mean decoding in the application, which is the boundary this design keeps.