// SPDX-License-Identifier: GPL-3.0-or-later
//! A receiver, running on a thread of its own.
//!
//! The window is not the receiver: the receiver is a Tokio future that owns sockets and
//! plays media, and the window must stay responsive while it runs. So a session starts one
//! thread, builds a Tokio runtime on it, builds the renderer, and serves until it is asked
//! to stop. Everything it has to say arrives on a channel.

pub mod config;
pub mod event;
pub mod logging;

use std::sync::Arc;

use async_channel::{Receiver, Sender};

use crate::render;
use crate::settings::Settings;
use crate::settings::identity::Identity;
use crate::settings::store::Paths;
use event::Event;
use logging::LogHandle;

/// A running receiver, or the attempt at one.
pub struct Session {
    events: Receiver<Event>,
    stop: Option<tokio::sync::oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl Session {
    /// Starts a receiver with these settings.
    ///
    /// The call returns as soon as the thread exists: whether the receiver started is an
    /// [Event::Started] or an [Event::Failed], not a return value. That is deliberate. A
    /// receiver that cannot bind its port fails a moment later, and the window must not
    /// freeze while a port is being tried.
    ///
    /// # Errors
    ///
    /// Returns the operating system's message when the thread cannot be started at all.
    pub fn start(
        settings: Settings,
        identity: Identity,
        paths: Paths,
        log: LogHandle,
    ) -> Result<Self, String> {
        let (events, event_receiver) = async_channel::unbounded();
        let (stop, stop_receiver) = tokio::sync::oneshot::channel();
        let thread = std::thread::Builder::new()
            .name("parlor-receiver".to_owned())
            .spawn(move || serve(settings, identity, paths, log, events, stop_receiver))
            .map_err(|error| error.to_string())?;
        Ok(Self { events: event_receiver, stop: Some(stop), thread: Some(thread) })
    }

    /// The channel an event arrives on.
    pub fn events(&self) -> &Receiver<Event> {
        &self.events
    }

    /// Asks the receiver to stop.
    ///
    /// The thread finishes on its own and reports [Event::Stopped]; nothing waits for it
    /// here, because the window would freeze for the length of a session's teardown.
    pub fn stop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        self.stop();
        if let Some(thread) = self.thread.take() {
            // A thread that is not joined is detached; it sees the stop signal and ends on
            // its own. Joining here would block the window for an unbounded time.
            drop(thread);
        }
    }
}

impl std::fmt::Debug for Session {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("Session").field("running", &self.stop.is_some()).finish()
    }
}

/// The thread: build a runtime, build the renderer, and serve.
fn serve(
    settings: Settings,
    identity: Identity,
    paths: Paths,
    log: LogHandle,
    events: Sender<Event>,
    stop: tokio::sync::oneshot::Receiver<()>,
) {
    let _ = events.try_send(Event::Starting);
    log.set_level(settings.diagnostics.log_level.filter());
    if let Err(error) = config::prepare_directories(&paths) {
        let _ = events.try_send(Event::Failed(format!("cannot write the log directory: {error}")));
        return;
    }
    let runtime =
        match tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build() {
            Ok(runtime) => runtime,
            Err(error) => {
                let _ =
                    events.try_send(Event::Failed(format!("cannot start the runtime: {error}")));
                return;
            }
        };
    let renderer: Arc<dyn rust_airplay::Renderer> = match render::build(&settings) {
        Ok(renderer) => renderer,
        Err(error) => {
            let _ = events.try_send(Event::Failed(error.to_string()));
            return;
        }
    };
    let config = config::receiver_config(&settings, &identity, renderer, &paths, &events);
    // The library binds its port inside `serve`, and reports a failure to bind as an
    // error. Announcing the configured port first is what lets the window show a running
    // receiver immediately; a port that cannot be bound replaces it with a failure a
    // moment later.
    let _ = events.try_send(Event::Started { port: settings.network.port });
    let shutdown = async move {
        let _ = stop.await;
    };
    let result = runtime.block_on(rust_airplay::serve(config, shutdown));
    match result {
        Ok(()) => {
            let _ = events.try_send(Event::Stopped);
        }
        Err(error) => {
            let _ = events.try_send(Event::Failed(error.to_string()));
        }
    }
}
#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::time::{Duration, Instant};

    use super::Session;
    use crate::session::event::Event;
    use crate::session::logging::LogHandle;
    use crate::settings::identity::Identity;
    use crate::settings::store::Paths;
    use crate::settings::{BackendKind, Settings};

    /// A port the system has just handed out and taken back.
    fn free_port() -> u16 {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("a free port");
        listener.local_addr().expect("an address").port()
    }

    #[test]
    #[ignore = "needs a playback backend and binds a local port"]
    fn a_session_answers_rtsp_on_its_port_and_stops() {
        let port = free_port();
        let directory = tempfile::tempdir().expect("a temporary directory");
        let paths = Paths::at(directory.path());
        let mut settings = Settings::new();
        settings.network.port = port;
        settings.network.bind = "127.0.0.1".to_owned();
        settings.network.advertise = false;
        settings.auth.pin = crate::settings::PinChoice::Disabled;
        settings.backend.kind = BackendKind::Ffmpeg;
        let mut session = Session::start(
            settings,
            Identity::test_identity([9; 32]),
            paths,
            LogHandle::detached(),
        )
        .expect("a receiver thread");

        let deadline = Instant::now() + Duration::from_secs(20);
        let mut started = false;
        while Instant::now() < deadline {
            match session.events().try_recv() {
                Ok(Event::Started { .. }) => {
                    started = true;
                    break;
                }
                Ok(Event::Failed(reason)) => panic!("the receiver failed: {reason}"),
                Ok(_) | Err(_) => std::thread::sleep(Duration::from_millis(50)),
            }
        }
        assert!(started, "the receiver did not report a port");

        let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("the control port");
        stream.set_read_timeout(Some(Duration::from_secs(5))).expect("a read timeout");
        stream
            .write_all(b"OPTIONS * RTSP/1.0\r\nCSeq: 1\r\nUser-Agent: ParlorTest/0.1\r\n\r\n")
            .expect("a request");
        let mut buffer = [0u8; 4096];
        let read = stream.read(&mut buffer).expect("a response");
        let response = String::from_utf8_lossy(&buffer[..read]);
        assert!(response.contains("RTSP/1.0 200"), "response: {response}");

        session.stop();
        let deadline = Instant::now() + Duration::from_secs(20);
        let mut stopped = false;
        while Instant::now() < deadline {
            match session.events().try_recv() {
                Ok(Event::Stopped) => {
                    stopped = true;
                    break;
                }
                Ok(Event::Failed(reason)) => panic!("the receiver failed: {reason}"),
                Ok(_) | Err(_) => std::thread::sleep(Duration::from_millis(50)),
            }
        }
        assert!(stopped, "the receiver did not report that it stopped");
    }

    #[test]
    #[ignore = "needs a playback backend and binds a local port"]
    fn a_port_in_use_is_reported_instead_of_serving() {
        let settings = Settings::new();
        if !crate::render::detect::probe(&settings).any_ready() {
            return;
        }
        let port = free_port();
        let held = std::net::TcpListener::bind(("127.0.0.1", port)).expect("a held port");
        let directory = tempfile::tempdir().expect("a temporary directory");
        let paths = Paths::at(directory.path());
        let mut settings = Settings::new();
        settings.network.port = port;
        settings.network.bind = "127.0.0.1".to_owned();
        settings.network.advertise = false;
        settings.auth.pin = crate::settings::PinChoice::Disabled;
        settings.backend.kind = BackendKind::Ffmpeg;
        let session = Session::start(
            settings,
            Identity::test_identity([8; 32]),
            paths,
            LogHandle::detached(),
        )
        .expect("a receiver thread");
        let deadline = Instant::now() + Duration::from_secs(20);
        let mut failure = None;
        while Instant::now() < deadline {
            match session.events().try_recv() {
                Ok(Event::Failed(reason)) => {
                    failure = Some(reason);
                    break;
                }
                Ok(Event::Started { .. }) => {
                    // The window is told the configured port as soon as the thread is up,
                    // so an optimistic Started may arrive before the bind is attempted.
                    // The failure that follows is what this test is about.
                }
                Ok(_) | Err(_) => std::thread::sleep(Duration::from_millis(50)),
            }
        }
        drop(held);
        let reason = failure.expect("a reported failure");
        assert!(reason.contains("bind"), "{reason}");
    }

    #[test]
    #[ignore = "needs the MSVC GStreamer runtime the scripts unpack"]
    fn a_gstreamer_session_starts_and_stops() {
        let launch = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../.tools/gstreamer/runtime/bin/gst-launch-1.0.exe");
        if !launch.is_file() {
            return;
        }
        let port = free_port();
        let directory = tempfile::tempdir().expect("a temporary directory");
        let paths = Paths::at(directory.path());
        let mut settings = Settings::new();
        settings.backend.kind = BackendKind::GStreamer;
        settings.backend.gstreamer.launch_path = launch.display().to_string();
        settings.network.port = port;
        settings.network.bind = "127.0.0.1".to_owned();
        settings.network.advertise = false;
        settings.auth.pin = crate::settings::PinChoice::Disabled;
        let mut session = Session::start(
            settings,
            Identity::test_identity([6; 32]),
            paths,
            LogHandle::detached(),
        )
        .expect("a receiver thread");
        let deadline = Instant::now() + Duration::from_secs(20);
        let mut started = false;
        while Instant::now() < deadline {
            match session.events().try_recv() {
                Ok(Event::Started { .. }) => {
                    started = true;
                    break;
                }
                Ok(Event::Failed(reason)) => panic!("the receiver failed: {reason}"),
                Ok(_) | Err(_) => std::thread::sleep(Duration::from_millis(50)),
            }
        }
        assert!(started, "the GStreamer session reported a port");
        session.stop();
        let deadline = Instant::now() + Duration::from_secs(20);
        while Instant::now() < deadline {
            match session.events().try_recv() {
                Ok(Event::Stopped) => return,
                Ok(Event::Failed(reason)) => panic!("the receiver failed: {reason}"),
                Ok(_) | Err(_) => std::thread::sleep(Duration::from_millis(50)),
            }
        }
        panic!("the GStreamer session did not stop");
    }
}
