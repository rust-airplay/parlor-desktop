# Feature inventory: rust-airplay 0.3.2

Scope: every value a desktop receiver UI can surface from `rust-airplay` 0.3.2, plus the
GStreamer backend (0.3.1) and the FFmpeg backend (0.1.0). Each claim names its source as
`file:line` relative to the crate named in the heading; nothing is inferred from behaviour
alone, and a default or range the source does not state is not listed. Sources read include
`src/receiver/{config,serve,runtime,state}.rs`, `src/server/{mod,pairing/state,pairing/mod}.rs`,
`src/server/handlers/{types,routes/helpers}.rs`, `src/media/ports/sockets.rs`,
`src/render/{codec,volume,traits}.rs`, `src/control/{mod,session}.rs`, both `docs/` sets, and
the `options`, `renderer`, `launch` and `sink` sources of the two backends.

The **When** column: `Restart` means the value is read once by `receiver::serve`
(`serve.rs:51-161`); `Live` means the library evaluates it while serving.

## 1. Capabilities a UI can expose

| Field | Type / variants | Default | Range, units | What it changes for the user | When | Proposed UI |
| --- | --- | --- | --- | --- | --- | --- |
| `Config::name` | `String` | required argument, no default | non-empty text | Name in the client's AirPlay menu, and the mDNS service name | Restart | Settings > Receiver |
| `Config::seed` | `[u8; 32]` | required argument, no default | 32 bytes | Every identity key derives from it; the same seed keeps the receiver recognisable (`API.md:76`) | Restart | Settings > Receiver, advanced (app stores it) |
| `Config::renderer` | `Arc<dyn Renderer>` | required argument | one of the backend renderers | Which backend plays audio and video | Restart | Settings > Backend |
| `Config::port` | `u16` | `7000` (`config.rs:207`) | `0..=65535`; `0` = an ephemeral port | TCP port of the control service | Restart | Settings > Network |
| `Config::bind` | `std::net::IpAddr` | `IpAddr::V4(0.0.0.0)` (`config.rs:208`) | IPv4/IPv6 address literal | Interface both control and media sockets bind to | Restart | Settings > Network |
| `Config::advertise` | `bool` | `true` (`config.rs:209`) | - | Whether the receiver registers mDNS records; off means the user types the address | Restart | Settings > Network |
| `Config::device_id` | `Option<String>` | `None` -> derived from seed (`config.rs:211`, `serve.rs:60-63`) | `aa:bb:cc:dd:ee:ff` | The `deviceID`/`macAddress` a client sees | Restart | Settings > Receiver, advanced |
| `Config::pin` | `PinMode` | `PinMode::OneTime` (`config.rs:212`) | `Disabled`, `OneTime`, `Fixed(u16)` | How a new client authenticates | Restart | Settings > Security |
| `Config::password` | `Option<String>` | `None` (`config.rs:213`) | any text | Digest password for `SETUP`, `SET_PARAMETER`, `GET_PARAMETER`, `FLUSH` (`API.md:79`) | Restart | Settings > Security |
| `Config::audio_codec` | `AudioCodec` | `AudioCodec::AacEld` (`config.rs:214`) | `Alac`, `AacEld` | Fallback coding when a client never states `ct`; a declaration always wins (`config.rs:62-68`) | Restart | Settings > Playback, advanced |
| `Config::initial_volume` | `Volume` | `Volume::FULL` = 1.0 (`config.rs:237`) | `0.0..=1.0` linear gain | Level the renderer is set to before any client connects | Restart | Settings > Playback |
| `Config::volume_curve.taper` | `bool` | `false` (`volume.rs:59`) | - | Flat vs tapered slider-to-gain mapping | Restart | Settings > Playback |
| `Config::volume_curve.low_db` | `f64` | `-30.0` (`volume.rs:60`) | decibels, no bound enforced | Gain at the bottom of a client's slider | Restart | Settings > Playback, advanced |
| `Config::volume_curve.high_db` | `f64` | `0.0` (`volume.rs:61`) | decibels, no bound enforced | Gain at the top of a client's slider | Restart | Settings > Playback, advanced |
| `Config::audio_latency` | `std::time::Duration` | `250 ms` = `DEFAULT_AUDIO_DELAY_MICROS` (`config.rs:222-224`) | `0..=10 s`, validated at startup (`serve.rs:111-119`) | The `Audio-Latency` a client schedules playback against | Restart | Settings > Playback |
| `Config::nohold` | `bool` | `false` (`config.rs:225`) | - | A new connection from the same host drops the previous one from that host | Restart | Settings > Receiver |
| `Config::feedback_timeout` | `Option<Duration>` | `None` (`config.rs:226`) | `None` or a positive `Duration` | Closes a session whose client stopped sending `/feedback`; `None` keeps a silent client | Restart | Settings > Receiver |
| `Config::ports.timing` | `u16` | `0` = system picks (`sockets.rs:46-51`) | `0..=65535` | UDP timing port named in `SETUP` | Restart | Settings > Network, advanced |
| `Config::ports.control` | `u16` | `0` | `0..=65535` | UDP control port, where resends arrive | Restart | Settings > Network, advanced |
| `Config::ports.data` | `u16` | `0` | `0..=65535` | UDP port audio datagrams arrive on | Restart | Settings > Network, advanced |
| `Config::ports.mirror` | `u16` | `0` | `0..=65535` | TCP listener the mirror stream connects to | Restart | Settings > Network, advanced |
| `Config::client_policy.restrict` | `bool` | `false` | - | Admit only devices in `allow`; a refused client gets `403 Forbidden` | Restart | Settings > Security |
| `Config::client_policy.allow` | `Vec<String>` | empty | `deviceID` strings | Devices admitted when `restrict` is set | Restart | Settings > Security |
| `Config::client_policy.block` | `Vec<String>` | empty | `deviceID` strings | Devices never admitted; block wins over allow | Restart | Settings > Security |
| `Config::clients` | `Arc<Mutex<HashSet<[u8; 32]>>>` | empty (`config.rs:217`) | 32-byte client keys | Recalled clients. With a PIN configured it decides whether a client may `/pair-verify` without pairing again; with the PIN disabled it is a record only (`serve.rs:71-80`) | Live | Settings > Security, known clients |
| `Config::on_client` | `Option<ClientCallback>` = `Arc<dyn Fn(&[u8; 32]) + Send + Sync>` | `None` | - | Tells the app to persist a newly verified key | Restart | Settings > Security, auto-remember |
| `Config::on_pin` | `Option<PinCallback>` = `Arc<dyn Fn(&str) + Send + Sync>` | `None` | 4-digit PIN text | The one thing a person must see: the pending PIN | Restart | Home, PIN dialog |
| `Config::features` | `Option<u64>` | `None` -> `DEFAULT_FEATURES` (`config.rs:229`, `serve.rs:88`) | any `u64`, no validation | The capability bitmap a client reads; see section 3 | Restart | Settings > Video & Display, advanced |
| `Config::display.width` | `u64` | `1920` | pixels, no bound enforced | Width a client is told its mirror should be | Restart | Settings > Video & Display |
| `Config::display.height` | `u64` | `1080` | pixels, no bound enforced | Height the client should send | Restart | Settings > Video & Display |
| `Config::display.refresh_rate` | `f64` | `60.0` | hertz; sent as the interval `1 / refresh_rate` | Refresh rate read from `/info` | Restart | Settings > Video & Display |
| `Config::display.max_fps` | `u64` | `30` | integer frames per second | Frame-rate cap a client is told to stay under | Restart | Settings > Video & Display |
| `Config::display.overscanned` | `bool` | `false` | - | Whether the advertised display is overscanned | Restart | Settings > Video & Display |
| `Config::pace_video` | `bool` | `false` (`config.rs:238`) | - | Hold mirror frames to capture spacing instead of showing the newest immediately | Restart | Settings > Playback, smooth video |
| `Config::debug_setup_body` | `bool` | `false` (`config.rs:215`) | - | Hands the whole `SETUP` body, including the media key, to the app | Restart | Settings > Diagnostics, unsafe |
| `Config::trace_log` | `Option<PathBuf>` | `None` (`config.rs:219`) | writable path | Appends a request trace; an unopenable path fails startup (`serve.rs:148-159`) | Restart | Settings > Diagnostics |
| `Config::dump_video` | `Option<PathBuf>` | `None` | writable path | Writes the first video frames, to diagnose a picture | Restart | Settings > Diagnostics |
| `Config::dump_audio` | `Option<PathBuf>` | `None` | writable path | Writes audio payloads, to diagnose a stream | Restart | Settings > Diagnostics |
| `Config::log_streaming_reports` | `bool` | `false` (`config.rs:235`) | - | Writes the client's once-a-second streaming reports to the log | Restart | Settings > Diagnostics |
| `Config::on_artwork` | `Option<ArtworkCallback>` = `Arc<dyn Fn(&str, &[u8]) + Send + Sync>` | `None` | content type and bytes | Now-playing artwork | Restart | Home |
| `Config::on_metadata` | `Option<MetadataCallback>` = `Arc<dyn Fn(&[u8]) + Send + Sync>` | `None` | raw DMAP listing | Title, artist, album, progress | Restart | Home |
| `Config::on_remote_control` | `Option<RemoteControlCallback>` = `Arc<dyn Fn(&str, &str) + Send + Sync>` | `None` | `DACP-ID`, `Active-Remote` | Identifiers a remote-control bridge needs | Restart | Settings > Diagnostics |
| GStream `Options::audio_sink` | `Option<String>` | `None` -> first installed of `AUDIO_SINKS` | sink fragment | Which sound device audio plays to | Restart | Settings > Backend |
| GStream `Options::video_sink` | `Option<String>` | `None` -> first installed of `VIDEO_SINKS` | sink fragment | Which window or output the mirror appears in | Restart | Settings > Backend |
| GStream `Options::h264` | `Chain` | `h264parse` + `avdec_h264` | parser and decoder names | How H.264 is decoded | Restart | Settings > Backend, advanced |
| GStream `Options::h265` | `Chain` | `h265parse` + `avdec_h265` | parser and decoder names | How HEVC is decoded | Restart | Settings > Backend, advanced |
| GStream `Options::warm_video` | `bool` | `false`, or `AIRPLAY_GST_VIDEO_WARM` | - | Whether a video window exists from receiver start | Restart | Settings > Backend |
| GStream `Options::pacing` | `bool` | `true` | - | Whether `set_pace` waiting applies; 0.3.2 never calls `set_pace` | Restart | Settings > Backend, advanced |
| FFmpeg `Options::decoder` | `String` | `"ffmpeg"`, or `AIRPLAY_FFMPEG` | executable name | Program that decodes audio | Restart | Settings > Backend |
| FFmpeg `Options::audio_sink` | `Program` | `ffplay -nodisp`, or `AIRPLAY_FFMPEG_AUDIO_SINK` | program plus arguments | Program that plays audio | Restart | Settings > Backend |
| FFmpeg `Options::video_sink` | `Program` | `ffplay -framedrop`, or `AIRPLAY_FFMPEG_VIDEO_SINK` | program plus arguments | Program that shows the mirror | Restart | Settings > Backend |
| FFmpeg `Options::pacing` | `bool` | `true` | - | Same inert hook as the GStreamer one | Restart | Settings > Backend, advanced |
| FFmpeg `Options::warm_video` | `bool` | `false` | - | Keeps the video sink running between sessions | Restart | Settings > Backend |
| FFmpeg `Options::warm_codec` | `VideoCodec` | `VideoCodec::H264` | `H264`, `H265` | Coding a warm sink starts for; a mismatch pays the start cost at the first frame | Restart | Settings > Backend, advanced |
| Client volume, runtime | `SetParameter::Volume(f32)` | renderer starts at `initial_volume` | decibels `-30..0`, `-144` mute | The volume the user moves on the phone, shown live | Live | Home |
| PIN, runtime | `PairingState::pending_pin() -> Option<String>` | `None` | 4 digits | Shown as a dialog while a client pairs | Live | Home, PIN dialog |

Fixed by the library, not reachable through `Config`: the advertised model `"AppleTV3,2"`,
`sourceVersion "220.68"`, `pi`, `vv`, `statusFlags 68`, `DISPLAY_FEATURES 14`,
`AUDIO_OUTPUT_FORMATS 0x03FF_FFFC` and `DISPLAY_UUID` (assembled in `serve.rs:87-96` with
`..DeviceIdentity::new(..)`), plus `MAX_CONNECTIONS = 12` and `DEFAULT_IDLE_TIMEOUT = 60 s`
(`server/mod.rs:62-71`), which `serve` takes from `ServerConfig::default()`.

## 2. Types and variants, verbatim

### PinMode (`server/pairing/state.rs:113-124`, `#[non_exhaustive]`)
    pub enum PinMode {
        Disabled,
        OneTime,
        Fixed(u16),
    }
`Disabled` refuses PIN pairing; `OneTime` is a PIN `/pair-pin-start` generates, valid until
one pairing succeeds; `Fixed(u16)` keeps only the low four digits (`pin % 10_000`,
`pairing/mod.rs:54-56`). `Config::new` sets `OneTime`.

### ClientPolicy (`handlers/types.rs:312-320`)
    pub struct ClientPolicy {
        pub restrict: bool,
        pub allow: Vec<String>,
        pub block: Vec<String>,
    }
`Default` derives to `false`, empty, empty. `admits` (`types.rs:325-336`): a `block` match is
refused first; with `restrict` set only an `allow` match is admitted; otherwise everything not
blocked is admitted. Matching is exact string equality against the client's `deviceID`.

### MediaPorts (`media/ports/sockets.rs:28-67`)
    pub struct MediaPorts {
        pub timing: u16,
        pub control: u16,
        pub data: u16,
        pub mirror: u16,
    }
Derives `Debug, Clone, Copy, PartialEq, Eq, Default`; `Default` is all zeroes, which is
`MediaPorts::automatic()`. `MediaPorts::legacy()` is `timing: 7011, control: 6001, data: 6000,
mirror: 7100`; the protocol's control port 7000 is `Config::port`, not one of the four.

### Display (`handlers/types.rs:345-369`)
    pub struct Display {
        pub width: u64,
        pub height: u64,
        pub refresh_rate: f64,
        pub max_fps: u64,
        pub overscanned: bool,
    }
`Default`: `1920`, `1080`, `60.0`, `30`, `false` (`DEFAULT_WIDTH`, `DEFAULT_HEIGHT`,
`DEFAULT_REFRESH_RATE`, `DEFAULT_MAX_FPS`). Sent as `displays[0]` with
`refreshRate: 1 / refresh_rate`, and `0.0` when the rate is not positive
(`routes/helpers.rs:88-112`).

### Volume and VolumeCurve (`render/volume.rs`)
    pub struct Volume(f64);                 // private field
    pub const FULL: Self = Self(1.0);
    pub struct VolumeCurve {
        pub taper: bool,     // default false
        pub low_db: f64,     // default -30.0
        pub high_db: f64,    // default 0.0
    }
`Volume::new(level)` returns `FULL` for `NaN` and clamps to `0.0..=1.0`; `fraction()` and
`level()` both return that number; `Default` is `FULL`; there are no other constants.
`db_for_fraction` clamps to `0..1`, returns `low_db` at 0, otherwise `low + (high - low) * f`;
with `taper` it is `high + 10 * log2(f)` max'd with the flat value. `gain(f)` is
`10^(0.05 * db_for_fraction(f))`, and `0.0` at `f <= 0`. The free function
`airplay_db_to_fraction(db)` maps `-144` and `-30` to `0.0`, `0` and above to `1.0`, linear
between.

### AudioCodec and AudioConfig (`render/codec.rs:16-107`, `#[non_exhaustive]`)
    pub enum AudioCodec {
        Alac,
        AacEld,
    }
`ALL = [Alac, AacEld]`; `codec_number()` is `1` / `3`; `from_compression_type` maps `2` to
`Alac` and `8` to `AacEld`, everything else `None`; `name()` is `"alac"` / `"aac-eld"`.
`AudioConfig { codec, sample_rate: u32, channels: u8, samples_per_frame: u16 }` defaults to
`Alac, 44_100, 2, 480`, which differs from `Config::audio_codec`.

### VideoCodec and VideoConfig (`render/codec.rs:109-148`, `#[non_exhaustive]`)
    pub enum VideoCodec {
        H264,   // #[default]
        H265,
    }
`name()` is `"h264"` / `"h265"`. `VideoConfig { width: Option<u32>, height: Option<u32>,
codec: VideoCodec }`, the dimensions `None` when the client did not state them.

Constants a UI shows or validates against: `DEFAULT_FEATURES = 0x527F_FEE6`, `DEFAULT_AUDIO_DELAY_MICROS
= 250_000`, `MAX_AUDIO_DELAY_MICROS = 10_000_000` (`control/mod.rs:69-72`), `AUDIO_SAMPLE_RATE =
44_100`, `MAX_CONNECTIONS = 12`, `DEFAULT_IDLE_TIMEOUT = 60 s`, `MAX_PIN_FAILURES = 5`,
`PIN_LOCKOUT = 30 s`, `PASSWORD_LOCKOUT = 30 s`, `MAX_DATAGRAM = 2048`.

## 3. The feature bitmap

`DEFAULT_FEATURES: u64 = 0x527F_FEE6` (`types.rs:206`), i.e. 1 384 120 038 decimal. Bits are
counted from the LSB as bit 0. Set in the default: 1, 2, 5, 6, 7, 9, 10, 11, 12, 13, 14, 15,
16, 17, 18, 19, 20, 21, 22, 25, 28, 30. Clear in the default: **4 (HLS)**, **27 (pairing
protocol)**, **42 (HEVC)**.

| Bit | Meaning | In the default | What a UI must know |
| --- | --- | --- | --- |
| 4 | HLS | clear | Advertising it does **not** enable HLS handling: `receiver::serve` never calls `ControlSession::set_hls_support` (`control/session.rs:50-56`), so the toggle alone is a promise the receiver does not keep |
| 5 | Slideshow | **set** | The crate has no slideshow route; the bit appears only in the `Config::features` doc comment |
| 27 | Pairing protocol | clear | Clear means AirPlay 2 client pairing. Set tells a client to use the older flow, which costs "the wait between tapping a receiver and seeing a picture" and does not complete here (`handlers/types.rs:187-206`) |
| 42 | HEVC video | clear | The only switch that makes a client send HEVC. With it set the mirror path sets `VideoCodec::H265` from the codec packet (`media/mirror/codec.rs:203`), so the backend must have an H.265 chain or the stream opens as `Unavailable` |

The protocol's options map as `-h265` = "this bitmap with bit 42 set" and `-hls` = bit 4
(`config.rs:121-128`). With the default as the base:

| Setting | Value | Decimal |
| --- | --- | --- |
| default (`features: None`) | `0x527FFEE6` | 1 384 120 038 |
| `-hls` (bit 4) | `0x527FFEF6` | 1 384 120 054 |
| `-h265` (bit 42) | `0x400527FFEE6` | 4 399 430 631 142 |
| both | `0x400527FFEF6` | 4 399 430 631 158 |

`features: Some(n)` replaces the bitmap outright instead of adding to it, and the library
validates nothing: a UI offering toggles must compute `DEFAULT_FEATURES | (1 << 42)` itself and
preserve the other bits.

## 4. The two backends

### 4.1 rust-airplay-gstream 0.3.1

**Needs native GStreamer at build time.** `Cargo.toml:50-53` depends on `gstreamer = "0.24"`
and `gstreamer-app = "0.24"`, which link `libgstreamer`; `docs/ARCHITECTURE.md:19-22` says so:
"building this crate needs the GStreamer development files while using it needs only the
runtime plugins". At run time it needs the GStreamer runtime and plugins for audio, and the
executable `gst-launch-1.0` for video (`launch.rs:9-13`, overridden by `AIRPLAY_GST_LAUNCH`).
Without it audio still plays: `with_options` logs "video will be unavailable" and builds
(`renderer.rs:104-117`). Audio is in-process through `appsrc`; video is that subprocess fed
Annex-B on standard input (`lib.rs:5-19`).

`Options` methods (`options/mod.rs:119-259`): `Default` (reads the environment), `new(audio_sink,
video_sink)` (environment ignored, warming off), `with_audio_sink`, `with_video_sink`,
`with_video_chain(codec, parser, decoder)`, `with_video_parser`, `with_video_decoder`,
`with_warm_video(bool)`, `with_pacing(bool)`, `without_pacing()`, and the getters
`audio_sink() -> Option<&str>`, `video_sink() -> Option<&str>`, `chain(codec) -> Result<&Chain,
RenderError>`, `warm_video() -> bool`, `pacing() -> bool`.

| UI field | Exact value | Default | Source |
| --- | --- | --- | --- |
| audio sink | fragment, one of `wasapisink`, `autoaudiosink`, `osssink`, `alsasink`, `pulsesink`, `fakesink` | first installed of that list | `options/mod.rs:18-25`, `sink.rs:56-74` |
| video sink | fragment, one of `d3d11videosink`, `autovideosink`, `glimagesink`, `fakesink` | first installed of that list | `options/mod.rs:28` |
| H.264 chain | `h264parse` / `avdec_h264` | those | `options/mod.rs:93-95` |
| H.265 chain | `h265parse` / `avdec_h265` | those | `options/mod.rs:98-100` |
| warm video | `true`/`false`; `AIRPLAY_GST_VIDEO_WARM` means on unless `0`, `false`, `no`, `off` | `false` | `options/mod.rs:126`, `269-281` |
| pacing | `true`/`false` | `true` | `options/mod.rs:127` |
| env sinks | `AIRPLAY_GST_AUDIO_SINK`, `AIRPLAY_GST_VIDEO_SINK` | unset | `options/mod.rs:31-34` |
| launcher | `AIRPLAY_GST_LAUNCH` | `gst-launch-1.0` | `pipeline/launch.rs:17` |
| keep stderr | `AIRPLAY_GST_KEEP_STDERR`, set to anything | unset: the temp file is deleted | `pipeline/launch.rs:25` |

A sink fragment is one element name plus `property=value` tokens; a fragment containing `!`,
`"` or `'`, or a token without `=`, is refused, and the element must be installed
(`options/sink.rs:22-48`). Guards: `MAX_PACE_SECONDS = 1.0`, `MAX_VIDEO_FRAME_BYTES =
16 * 1024 * 1024` (`options/mod.rs:46-53`).

### 4.2 rust-airplay-ffmpeg 0.1.0

**No native libraries at build time.** `Cargo.toml:42-48` depends only on `rust-airplay =
"0.3.2"` and `log`; `docs/ARCHITECTURE.md:164`: "No C library, no build-time FFmpeg, no `-sys`
crate". At run time it needs **executables only**: `ffmpeg` to decode audio and `ffplay` to
play, both probed in `with_options` (`renderer.rs:71-76`), so a machine without them is refused
at startup with `RenderError::Unavailable` instead of at the first frame (`lib.rs:35-38`).
Audio is decoded through a live Matroska stream to PCM; video is Annex-B written to the sink.

`Options` accessors (`options.rs:168-303`): `Default`, `new(audio_sink: Program, video_sink:
Program)`, `with_decoder`, `with_audio_sink`, `with_video_sink`, `with_warm_video`,
`warm_video()`, `with_warm_codec`, `warm_codec()`, `with_pacing`, `without_pacing`,
`pacing()`, `decoder()`, `audio_sink() -> &Program`, `video_sink() -> &Program`. `Program`
builds a command line: `new(name)`, `raw(name)`, `arg`, `args`, `with_env`, plus `name()`,
`arguments()`, `environment()`, `dialect()` (`Dialect::Ffmpeg` | `Dialect::Raw`).

| UI field | Exact value | Default | Source |
| --- | --- | --- | --- |
| decoder | `"ffmpeg"`, or `AIRPLAY_FFMPEG` | `"ffmpeg"` | `options.rs:7`, `18`, `171` |
| audio sink | `ffplay -nodisp`, or `AIRPLAY_FFMPEG_AUDIO_SINK` | `ffplay -nodisp` | `options.rs:15`, `21`, `174-175` |
| video sink | `ffplay -framedrop`, or `AIRPLAY_FFMPEG_VIDEO_SINK` | `ffplay -framedrop` | `options.rs:24`, `179-180` |
| pacing | `true`/`false` | `true` | `options.rs:181` |
| warm video | `true`/`false` | `false` | `options.rs:182` |
| warm codec | `H264`/`H265` | `H264` | `options.rs:183` |

An environment sink is split on whitespace with quotes not interpreted, so a path with a space
cannot be set that way and must go through `Options`/`Program` (`options.rs:314-329`).
`MAX_PACE_SECONDS = 1.0` (`options.rs:31`).

**Which needs what:** GStreamer needs development files at build time plus its runtime and
`gst-launch-1.0` at run time; FFmpeg needs nothing but a Rust toolchain at build time and the
`ffmpeg` and `ffplay` executables at run time. Both satisfy one `rust-airplay` 0.3.2 in the
graph: 0.3.1 declares `"0.3.1"` and 0.1.0 declares `"0.3.2"`, both caret ranges below 0.4.

## 5. Constraints a UI must respect

- `Display::max_fps` is `u64`, not `f64` (`types.rs:354`); a fractional cap will not compile.
  `Display::refresh_rate` is `f64` in hertz and is sent as the interval `1 / refresh_rate`
  (`types.rs:351`, `routes/helpers.rs:88-96`). A non-positive rate is tolerated as interval
  `0.0`, so the UI must reject `0` and negatives itself.
- Width and height are `u64` pixels with no bound or evenness check; a client sizes its mirror
  from whatever it is told.
- Ports are `u16`: `0..=65535`, and `0` means the system picks, for `Config::port` and each
  `MediaPorts` field. The assigned control port is only logged and advertised, never returned
  by `serve`, so a UI that shows the address must require a fixed non-zero port.
- `Config::bind` is a bare `std::net::IpAddr`, parsed as an address literal: `"localhost"` and
  `"192.168.1.5:7000"` must be rejected before assignment. All four media ports bind on that
  same address (`serve.rs:216-221`); a fixed port in use fails startup with "cannot bind the
  media ports". The legacy layout is `7011, 6001, 6000, 7100`.
- `audio_latency` is validated as `0..=MAX_AUDIO_DELAY_MICROS` **before** the listener binds
  (`serve.rs:111-119`), so a value outside it fails startup with a message; clamp the UI to
  `0..=10` seconds. It is a `Duration`, converted to `u32` microseconds.
- `feedback_timeout`: `None` means a silent client is never closed, which is this crate's
  default (`config.rs:103-107`). `Some(Duration::ZERO)` closes on the first check
  (`connection.rs:117-123`), so require a positive value; the protocol's `-reset` is 15 s.
- `nohold` is weaker than it sounds: the receiver already serves up to 12 connections at once
  and `nohold` only aborts the previous connection **from the same host**
  (`server/mod.rs:88-96`, `250-259`). It does not keep two clients from replacing each other's
  media: there is one media session, and "the latest `SETUP` that carries a key or mirror
  identifier replaces the current one" (`API.md:34-37`).
- `Config::clients` is a `std::sync::Mutex<HashSet<[u8; 32]>>`, not a tokio lock: hold it only
  for short non-async sections and expect the library to lock it from network threads. The
  keys are raw 32-byte arrays, so persistence needs an encoding the app chooses.
- `PinMode::Fixed(u16)` keeps only the low four digits (`pin % 10_000`); `OneTime` is consumed
  when a pairing succeeds; `MAX_PIN_FAILURES = 5` wrong proofs drop it and start
  `PIN_LOCKOUT = 30 s`. A password has the same bound with `PASSWORD_LOCKOUT = 30 s`.
- Callbacks run on library threads and must be marshalled to the UI thread: `on_pin` from a
  dedicated `std::thread` polling every 500 ms (`serve.rs:184-202`), `on_client` from the
  control connection inside `SETUP` (`serve.rs:625-633`), and `on_artwork`/`on_metadata`/
  `on_remote_control` from the `SET_PARAMETER` path (`handlers/routes/parameters.rs:16-27`).
  `API.md:104-106` says slow work belongs elsewhere; in a desktop app that means a channel and
  an emit, never a direct widget call.
- `Config` is `#[non_exhaustive]` (`config.rs:32`), and so are `PinMode`, `AudioCodec` and
  `VideoCodec`: build from `Config::new` plus field assignment, and match with a wildcard arm.
- `features` is unvalidated and absolute; the UI must compute the whole bitmap and should warn
  that bit 42 has to match what the chosen backend can decode.
- Pacing has two switches that do not stack: `Config::pace_video` makes the receiver wait, and
  each backend's `pacing` only affects `VideoStream::set_pace`, which 0.3.2 never calls
  (`BACKEND.md:99-110`, `render/traits.rs:96-100`). Offer one "smooth video" toggle for
  `pace_video` and leave backend pacing alone.
- `initial_volume` is a linear gain applied once, before any client; the value advertised in
  `/info` is the matching **decibel** number from the curve (`serve.rs:94`, `runtime.rs:223-225`).
  A slider that calls `Renderer::set_volume` changes the live gain but is overwritten by the
  next client `SET_PARAMETER volume:`, and it does not change the advertised `initialVolume`.
- A client's volume may arrive as decibels (`-30..0`, `-144` mute) or, when positive, as a
  fraction `0..1` or a percentage `0..100` (`runtime.rs:203-220`); a "50%" readout must not
  assume the client sent decibels.
- The GStreamer backend refuses an audio configuration its caps cannot express, including a
  non-stereo one (`gstream/src/audio/graph.rs` and its tests). No `Config` field exposes
  channels today.

## 6. Behaviour the library leaves to the program

- **Identity seed storage.** `Config::seed` is 32 bytes the caller supplies; the library reads
  no file and no environment (`config.rs:15-18`, `API.md:66-68`). Keeping it is what makes a
  receiver recognisable after a restart; losing it makes every client pair again.
- **Device identifier.** `device_id: None` derives a stable address from the seed (SHA-512 of
  `"rust-airplay-device-id"` plus the seed, first six bytes, locally-administered bit forced,
  `receiver/state.rs:20-30`). Showing or overriding it is the app's choice.
- **Client registry.** `clients` is an in-memory set; persisting, pruning and displaying it are
  the app's job, and `on_client` is the only notice that a key is new.
- **PIN presentation.** `on_pin` delivers four digits and nothing else: no expiry countdown
  beyond the 30 s lockout, no wrong-PIN feedback, no dismissal.
- **Artwork and metadata.** `on_artwork` hands over a content type and bytes, `on_metadata` a
  raw DMAP listing that `media::dmap::text` turns into lines. The now-playing panel, caching
  and image decoding are the app's.
- **Remote-control identifiers.** `on_remote_control` receives `DACP-ID` and
  `Active-Remote`; they "authorize nothing; they name a connection" (`config.rs:152-156`).
- **Trace and dump paths.** `trace_log`, `dump_video` and `dump_audio` are caller-named paths
  opened for append; nothing rotates, truncates or cleans them, and a trace path that cannot be
  opened fails startup (`serve.rs:148-159`).
- **Logging.** The crate only writes `log` records and nothing appears without a logger
  (`API.md:216-224`). Installing a subscriber, choosing a level and routing it to a UI log pane
  are the app's; `log_streaming_reports` only decides whether one kind of record is written.
- **Start and stop lifetime.** `receiver::serve` runs until the caller's shutdown future
  completes, then returns after the control service stopped and the advertisement was
  withdrawn; `run` is just Ctrl+C (`serve.rs:20-58`). The app owns the task and the shutdown
  signal, and the error surface: a startup failure is the returned `Error`, while a session
  that fails later is logged and must not stop the receiver (`API.md:39-47`).
