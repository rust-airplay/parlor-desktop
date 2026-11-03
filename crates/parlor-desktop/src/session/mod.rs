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
