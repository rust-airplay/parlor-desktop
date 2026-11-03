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
    config.display = settings.display.display();
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
}
