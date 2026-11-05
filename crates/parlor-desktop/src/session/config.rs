// SPDX-License-Identifier: GPL-3.0-or-later
//! What the settings mean to the receiver, assembled once per session.
//!
//! The receiver reads its configuration once and never again, so this module is the only
//! place where a setting becomes a value the library sees. Nothing here opens a socket;
//! it names paths, counts callbacks and hands over the renderer.

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use async_channel::Sender;
use rust_airplay::{Config, Renderer};

use crate::session::event::{self, Event};
use crate::settings::Settings;
use crate::settings::identity::Identity;
use crate::settings::store::Paths;

/// The configuration a session starts with.
///
/// Every callback is a one-channel send: the library calls them from threads it owns, and
/// the window reads the channel on its own thread.
pub fn receiver_config(
    settings: &Settings,
    identity: &Identity,
    renderer: Arc<dyn Renderer>,
    paths: &Paths,
    events: &Sender<Event>,
) -> Config {
    let mut config = Config::new(settings.receiver.name(), identity.seed(), renderer);
    config.port = settings.network.port;
    config.bind = settings
        .network
        .bind_address()
        .unwrap_or(std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED));
    config.advertise = settings.network.advertise;
    config.device_id = settings.receiver.device_id();
    config.pin = settings.auth.pin_mode();
    config.password = settings.auth.password();
    config.audio_codec = settings.audio.codec();
    config.debug_setup_body = settings.diagnostics.setup_body;
    config.audio_latency = settings.audio.latency();
    config.nohold = settings.receiver.nohold;
    config.feedback_timeout = settings
        .receiver
        .feedback_timeout
        .map(|seconds| std::time::Duration::from_secs(u64::from(seconds)));
    config.ports = settings.network.media_ports();
    config.client_policy = settings.auth.client_policy();
    config.features = Some(settings.video.features());
    // What the client is told about the display. A mirror is the device's own picture scaled
    // to this size, so asking for more than 1080p adds load without adding detail, and a
    // frame rate past 60 is motion no mirror client can encode well. The FFmpeg backend
    // carries raw frames through a pipe and cannot keep up with a 4K stream at all.
    let mut display = settings.display.display();
    display.max_fps = display.max_fps.min(60);
    display.refresh_rate = display.refresh_rate.min(60.0);
    // Software decoding is the reason for the cap; a hardware decoder can carry the size
    // the person chose.
    if settings.backend.kind == crate::settings::BackendKind::Ffmpeg
        && !settings.backend.ffmpeg.hardware
        && (display.width > 1920 || display.height > 1080)
    {
        let scale = (1920.0 / display.width as f64).min(1080.0 / display.height as f64);
        display.width = (display.width as f64 * scale).round() as u64;
        display.height = (display.height as f64 * scale).round() as u64;
    }
    log::info!(
        "display {}x{} at {:.0} Hz, at most {} fps (asked {}x{} at {:.0} Hz, at most {} fps)",
        display.width,
        display.height,
        display.refresh_rate,
        display.max_fps,
        settings.display.width,
        settings.display.height,
        settings.display.refresh_rate,
        settings.display.max_fps
    );
    config.display = display;
    config.log_streaming_reports = settings.diagnostics.streaming_reports;
    config.volume_curve = settings.audio.volume_curve();
    config.initial_volume = settings.audio.initial_volume();
    config.pace_video = settings.receiver.pace_video;
    config.trace_log = enabled(settings.diagnostics.trace_log).then(|| paths.trace_log());
    config.dump_video = enabled(settings.diagnostics.dump_video).then(|| paths.video_dump());
    config.dump_audio = enabled(settings.diagnostics.dump_audio).then(|| paths.audio_dump());
    if settings.auth.remember_clients {
        config.clients = Arc::new(Mutex::new(remembered(identity)));
    }

    let pin_events = events.clone();
    config.on_pin = Some(Arc::new(move |pin: &str| {
        let _ = pin_events.try_send(Event::Pin(pin.to_owned()));
    }));

    let client_events = events.clone();
    config.on_client = Some(Arc::new(move |key: &[u8; 32]| {
        let _ = client_events.try_send(Event::Client(*key));
    }));

    let artwork_events = events.clone();
    let artwork_paths = paths.clone();
    config.on_artwork = Some(Arc::new(move |content_type: &str, bytes: &[u8]| {
        if let Some(path) = event::write_artwork(&artwork_paths, content_type, bytes) {
            let _ = artwork_events
                .try_send(Event::Artwork { content_type: content_type.to_owned(), path });
        }
    }));

    let metadata_events = events.clone();
    config.on_metadata = Some(Arc::new(move |bytes: &[u8]| {
        let text = rust_airplay::media::dmap::text(bytes);
        let lines: Vec<String> = text.lines().map(str::to_owned).collect();
        if !lines.is_empty() {
            let _ = metadata_events.try_send(Event::Metadata(lines));
        }
    }));

    let remote_events = events.clone();
    config.on_remote_control = Some(Arc::new(move |dacp_id: &str, active_remote: &str| {
        let _ = remote_events.try_send(Event::RemoteControl {
            dacp_id: dacp_id.to_owned(),
            active_remote: active_remote.to_owned(),
        });
    }));

    config
}

/// The clients a previous session verified.
fn remembered(identity: &Identity) -> HashSet<[u8; 32]> {
    identity.clients().iter().copied().collect()
}

/// The directories a diagnostic file is written to, created before the receiver opens it.
///
/// The receiver fails to start when a trace file cannot be opened, so a directory that
/// does not exist yet would turn a diagnostic switch into a session that never starts.
pub fn prepare_directories(paths: &Paths) -> std::io::Result<()> {
    std::fs::create_dir_all(paths.logs())
}

/// Whether a switch that writes a file is on.
fn enabled(flag: bool) -> bool {
    flag
}

#[cfg(test)]
mod tests {
    use super::receiver_config;
    use crate::session::event::Event;
    use crate::settings::Settings;
    use crate::settings::identity::Identity;
    use crate::settings::store::Paths;
    use rust_airplay::NullRenderer;
    use std::sync::Arc;

    fn settings_and_identity() -> (Settings, Identity, Paths) {
        let directory = std::env::temp_dir().join("parlor-config-test");
        (Settings::new(), Identity::test_identity([7; 32]), Paths::at(directory))
    }

    #[test]
    fn the_settings_reach_the_receiver() {
        let (mut settings, identity, paths) = settings_and_identity();
        settings.network.port = 7100;
        settings.receiver.name = "Study".to_owned();
        settings.auth.pin = crate::settings::PinChoice::Fixed;
        settings.auth.pin_digits = 42;
        let (sender, _receiver) = async_channel::unbounded();
        let renderer: Arc<dyn rust_airplay::Renderer> = Arc::new(NullRenderer::new());
        let config = receiver_config(&settings, &identity, renderer, &paths, &sender);
        assert_eq!(config.port, 7100);
        assert_eq!(config.name, "Study");
        assert_eq!(config.pin, rust_airplay::server::PinMode::Fixed(42));
        assert_eq!(config.features, Some(settings.video.features()));
        assert_eq!(config.display.width, 1920);
        assert!(config.trace_log.is_none());
    }

    #[test]
    fn a_diagnostic_switch_names_a_file() {
        let (mut settings, identity, paths) = settings_and_identity();
        settings.diagnostics.trace_log = true;
        let (sender, _receiver) = async_channel::unbounded();
        let renderer: Arc<dyn rust_airplay::Renderer> = Arc::new(NullRenderer::new());
        let config = receiver_config(&settings, &identity, renderer, &paths, &sender);
        assert_eq!(config.trace_log, Some(paths.trace_log()));
    }

    #[test]
    fn a_pin_reaches_the_window() {
        let (settings, identity, paths) = settings_and_identity();
        let (sender, receiver) = async_channel::unbounded();
        let renderer: Arc<dyn rust_airplay::Renderer> = Arc::new(NullRenderer::new());
        let config = receiver_config(&settings, &identity, renderer, &paths, &sender);
        let callback = config.on_pin.clone().expect("a PIN callback");
        callback("1234");
        assert_eq!(receiver.try_recv(), Ok(Event::Pin("1234".to_owned())));
    }

    #[test]
    fn the_advertised_display_stays_within_what_a_mirror_carries() {
        let (mut settings, identity, paths) = settings_and_identity();
        let (sender, _receiver) = async_channel::unbounded();
        let renderer: Arc<dyn rust_airplay::Renderer> = Arc::new(NullRenderer::new());

        settings.display.width = 3840;
        settings.display.height = 2160;
        settings.display.max_fps = 120;
        settings.display.refresh_rate = 120.0;
        let config = receiver_config(&settings, &identity, Arc::clone(&renderer), &paths, &sender);
        // A GStreamer mirror keeps the size the person asked for, but not a rate no mirror
        // client encodes well.
        assert_eq!((config.display.width, config.display.height), (3840, 2160));
        assert_eq!(config.display.max_fps, 60);
        assert_eq!(config.display.refresh_rate, 60.0);

        // The FFmpeg backend carries raw frames through a pipe; 4K is clamped to 1080p,
        // which still has the same shape.
        settings.backend.kind = crate::settings::BackendKind::Ffmpeg;
        settings.backend.ffmpeg.hardware = false;
        let config = receiver_config(&settings, &identity, Arc::clone(&renderer), &paths, &sender);
        assert_eq!((config.display.width, config.display.height), (1920, 1080));

        // With the GPU decoder the chosen size is kept.
        settings.backend.ffmpeg.hardware = true;
        let config = receiver_config(&settings, &identity, renderer, &paths, &sender);
        assert_eq!((config.display.width, config.display.height), (3840, 2160));
    }

    #[test]
    fn every_setting_reaches_the_receiver() {
        let (mut settings, mut identity, paths) = settings_and_identity();
        identity.remember([7; 32]);
        settings.receiver.name = "Study".to_owned();
        settings.receiver.device_id = "aa:bb:cc:dd:ee:ff".to_owned();
        settings.receiver.pace_video = true;
        settings.receiver.nohold = true;
        settings.receiver.feedback_timeout = Some(9);
        settings.display.width = 2560;
        settings.display.height = 1440;
        settings.display.refresh_rate = 120.0;
        settings.display.max_fps = 60;
        settings.display.overscanned = true;
        settings.auth.pin = crate::settings::PinChoice::Fixed;
        settings.auth.pin_digits = 4321;
        settings.auth.password = "secret".to_owned();
        settings.auth.restrict = true;
        settings.auth.allow = vec!["one".to_owned()];
        settings.auth.block = vec!["two".to_owned()];
        settings.network.port = 7199;
        settings.network.bind = "127.0.0.1".to_owned();
        settings.network.advertise = false;
        settings.network.ports = crate::settings::PortsMode::Custom;
        settings.network.timing = 7011;
        settings.network.control = 6001;
        settings.network.data = 6000;
        settings.network.mirror = 7100;
        settings.video.hevc = true;
        settings.video.hls = true;
        settings.video.slideshow = false;
        settings.audio.codec = crate::settings::AudioCodecChoice::Alac;
        settings.audio.latency_ms = 123;
        settings.audio.initial_volume = 40;
        settings.audio.taper = true;
        settings.audio.low_db = -20.0;
        settings.audio.high_db = -2.0;
        settings.diagnostics.streaming_reports = true;
        settings.diagnostics.setup_body = true;
        settings.diagnostics.trace_log = true;
        settings.diagnostics.dump_video = true;
        settings.diagnostics.dump_audio = true;
        let (sender, _receiver) = async_channel::unbounded();
        let renderer: Arc<dyn rust_airplay::Renderer> = Arc::new(NullRenderer::new());
        let config = receiver_config(&settings, &identity, renderer, &paths, &sender);

        assert_eq!(config.name, "Study");
        assert_eq!(config.device_id.as_deref(), Some("aa:bb:cc:dd:ee:ff"));
        assert!(config.pace_video);
        assert!(config.nohold);
        assert_eq!(config.feedback_timeout, Some(std::time::Duration::from_secs(9)));
        assert_eq!(config.port, 7199);
        assert_eq!(config.bind, std::net::IpAddr::from([127, 0, 0, 1]));
        assert!(!config.advertise);
        assert_eq!(config.display.width, 2560);
        assert_eq!(config.display.height, 1440);
        // The advertised rate is capped at 60: see the display clamp in receiver_config.
        assert_eq!(config.display.refresh_rate, 60.0);
        assert_eq!(config.display.max_fps, 60);
        assert!(config.display.overscanned);
        assert_eq!(config.pin, rust_airplay::server::PinMode::Fixed(4321));
        assert_eq!(config.password.as_deref(), Some("secret"));
        assert!(config.client_policy.restrict);
        assert_eq!(config.client_policy.allow, vec!["one".to_owned()]);
        assert_eq!(config.client_policy.block, vec!["two".to_owned()]);
        assert_eq!(config.audio_codec, rust_airplay::AudioCodec::Alac);
        assert_eq!(config.audio_latency, std::time::Duration::from_millis(123));
        assert_eq!(config.ports.timing, 7011);
        assert_eq!(config.ports.control, 6001);
        assert_eq!(config.ports.data, 6000);
        assert_eq!(config.ports.mirror, 7100);
        let features = config.features.expect("a bitmap");
        assert_eq!(features & (1 << 42), 1 << 42, "HEVC is offered");
        assert_eq!(features & (1 << 4), 1 << 4, "HLS is offered");
        assert_eq!(features & (1 << 5), 0, "slideshow is not offered");
        assert!(config.log_streaming_reports);
        assert!(config.debug_setup_body);
        assert!(config.volume_curve.taper);
        assert_eq!(config.volume_curve.low_db, -20.0);
        assert_eq!(config.volume_curve.high_db, -2.0);
        assert!((config.initial_volume.fraction() - 0.4).abs() < 1e-9);
        assert_eq!(config.trace_log.as_deref(), Some(paths.trace_log().as_path()));
        assert_eq!(config.dump_video.as_deref(), Some(paths.video_dump().as_path()));
        assert_eq!(config.dump_audio.as_deref(), Some(paths.audio_dump().as_path()));
        assert!(config.clients.lock().expect("the client list").contains(&[7; 32]));
        assert!(config.on_pin.is_some());
        assert!(config.on_client.is_some());
        assert!(config.on_artwork.is_some());
        assert!(config.on_metadata.is_some());
        assert!(config.on_remote_control.is_some());
    }
}
