// SPDX-License-Identifier: GPL-3.0-or-later
//! The GStreamer backend, loaded from a plugin so the application still starts without it.
//!
//! GStreamer is a native library. Linking it into the application would make the
//! application itself refuse to start on a machine that has no GStreamer, and a program
//! that cannot start cannot offer to install what it needs. So the backend is a separate
//! library, [parlor-gst-backend](parlor_gst_backend), opened here on demand.
//!
#![allow(unsafe_code)] // Opening a native library is the whole reason this module exists.

//! The plugin is never unloaded. A renderer keeps code pointers into it for as long as a
//! session lives, and Windows does not let the application know when the last of those is
//! gone; keeping one library mapped for the life of the process costs nothing that matters.

use std::sync::Arc;

use parlor_backend_api::GStreamerOptions;
use rust_airplay::Renderer;

use crate::render::Error;
use crate::render::detect;
use crate::settings::Settings;

/// The name of the variable the backend reads for the program that runs pipelines.
const LAUNCH_ENV: &str = "AIRPLAY_GST_LAUNCH";

/// Builds a GStreamer renderer through the plugin.
///
/// # Errors
///
/// Returns [Error::GStreamer] when the plugin is missing, cannot be loaded, was built for
/// another ABI, or refuses the options.
pub fn build(settings: &Settings) -> Result<Arc<dyn Renderer>, Error> {
    let plugin_path = detect::plugin_path().filter(|path| path.is_file()).ok_or_else(|| {
        let expected = detect::plugin_path().map(|path| path.display().to_string());
        Error::GStreamer(match expected {
            Some(expected) => format!("the plugin was not found at {expected}"),
            None => "the plugin was not found beside the application".to_owned(),
        })
    })?;
    // The plugin imports the GStreamer DLLs, so the runtime's own directory has to be
    // reachable before the plugin is opened. It is not enough to find gst-launch-1.0: a
    // runtime in a portable folder or on the PATH puts its DLLs beside that program, and
    // LoadLibraryEx does not search the PATH once the search flags are in use.
    let launch = detect::gst_launch(settings);
    let mut directories = detect::plugin_directories();
    if let Some(launch) = launch.as_ref()
        && let Some(directory) = launch.parent()
    {
        directories.push(directory.to_path_buf());
    }
    for directory in &directories {
        platform::add_dll_directory(directory);
    }
    if let Some(launch) = launch {
        // GStreamer loads its plugins with the altered search path, which consults PATH and
        // not the directories AddDllDirectory recorded. Without the runtime on PATH a plugin
        // such as gstlibav cannot find avcodec, so the runtime's bin goes to the front.
        if let Some(directory) = launch.parent() {
            platform::prepend_path(directory);
        }
        set_launch_program(&launch);
    }
    let options = GStreamerOptions {
        audio_sink: settings.backend.gstreamer.audio_sink.clone(),
        video_sink: settings.backend.gstreamer.video_sink.clone(),
        h264_parser: settings.backend.gstreamer.h264_parser.clone(),
        h264_decoder: settings.backend.gstreamer.h264_decoder.clone(),
        h265_parser: settings.backend.gstreamer.h265_parser.clone(),
        h265_decoder: settings.backend.gstreamer.h265_decoder.clone(),
        warm_video: settings.backend.gstreamer.warm_video,
        pacing: settings.backend.gstreamer.pacing,
    };
    let document = serde_json::to_vec(&options)
        .map_err(|error| Error::GStreamer(format!("cannot describe the options: {error}")))?;
    let library = platform::Library::open(&plugin_path).map_err(Error::GStreamer)?;
    let library = Box::leak(Box::new(library));
    library.create(&document).map_err(Error::GStreamer)
}

/// Tells the backend where gst-launch-1.0 is.
///
/// The backend reads this variable when it opens a video stream, and the variable is the
/// only way it accepts a program outside the PATH. Setting it before the renderer is built
/// is therefore the only ordering that works.
fn set_launch_program(launch: &std::path::Path) {
    // SAFETY: the variable is set before any session exists, and the only reader is the
    // backend, which reads it when a stream opens. No other thread reads this name.
    unsafe {
        std::env::set_var(LAUNCH_ENV, launch);
    }
}

#[cfg(target_os = "windows")]
mod platform {
    use std::ffi::{CStr, c_void};
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;
    use std::sync::Arc;

    use parlor_backend_api::{
        ABI_SYMBOL, ABI_VERSION, CREATE_SYMBOL, CreateFn, ERROR_SYMBOL, ErrorFn,
    };
    use rust_airplay::Renderer;
    use windows_sys::Win32::Foundation::{GetLastError, HMODULE};
    use windows_sys::Win32::System::LibraryLoader::{
        AddDllDirectory, GetProcAddress, LOAD_LIBRARY_SEARCH_DEFAULT_DIRS,
        LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR, LoadLibraryExW,
    };

    /// A plugin that stays mapped for the life of the process.
    pub struct Library {
        module: HMODULE,
    }

    impl Library {
        /// Opens the plugin, resolving its imports from the directories added so far.
        pub fn open(path: &Path) -> Result<Self, String> {
            let wide: Vec<u16> = path.as_os_str().encode_wide().chain([0]).collect();
            // SAFETY: the path is a null-terminated UTF-16 string that outlives the call.
            let module = unsafe {
                LoadLibraryExW(
                    wide.as_ptr(),
                    std::ptr::null_mut(),
                    LOAD_LIBRARY_SEARCH_DEFAULT_DIRS | LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR,
                )
            };
            if module.is_null() {
                // SAFETY: the previous call is the one that failed.
                let code = unsafe { GetLastError() };
                // 126 is ERROR_MOD_NOT_FOUND: the plugin itself was found, but one of the
                // libraries it imports was not. That is either a runtime that is not the
                // MSVC build, or a runtime whose directory was not added.
                let hint = if code == 126 {
                    " (a library the plugin imports was not found; the GStreamer runtime has to be the MSVC build and lie beside the gst-launch-1.0 the settings name)"
                } else {
                    ""
                };
                return Err(format!("cannot load {}: Windows error {code}{hint}", path.display()));
            }
            Ok(Self { module })
        }

        /// The ABI the plugin was built for.
        fn abi(&self) -> Option<u32> {
            // SAFETY: the symbol name is a static C string, and the plugin is loaded.
            let symbol = unsafe { self.symbol(ABI_SYMBOL) }?;
            // SAFETY: the plugin exports this symbol with the signature the ABI names.
            let abi: unsafe extern "C" fn() -> u32 = unsafe { std::mem::transmute(symbol) };
            // SAFETY: the symbol was checked to exist and has the ABI's signature.
            Some(unsafe { abi() })
        }

        /// The last failure the plugin recorded.
        fn last_error(&self) -> Option<String> {
            // SAFETY: the symbol name is a static C string, and the plugin is loaded.
            let symbol = unsafe { self.symbol(ERROR_SYMBOL) }?;
            // SAFETY: the plugin exports this symbol with the signature the ABI names.
            let error: ErrorFn = unsafe { std::mem::transmute(symbol) };
            // SAFETY: the plugin returns null or a pointer to a string it owns until the
            // next create call, which has already happened.
            let pointer = unsafe { error() };
            if pointer.is_null() {
                return None;
            }
            Some(unsafe { CStr::from_ptr(pointer) }.to_string_lossy().into_owned())
        }

        /// Creates a renderer through the plugin.
        pub fn create(&self, document: &[u8]) -> Result<Arc<dyn Renderer>, String> {
            match self.abi() {
                Some(version) if version == ABI_VERSION => {}
                Some(version) => {
                    return Err(format!(
                        "the plugin speaks ABI {version}, this application speaks {ABI_VERSION}"
                    ));
                }
                None => return Err("the plugin does not report an ABI".to_owned()),
            }
            // SAFETY: the symbol name is a static C string, and the plugin is loaded.
            let symbol = unsafe { self.symbol(CREATE_SYMBOL) }
                .ok_or_else(|| "the plugin has no create symbol".to_owned())?;
            // SAFETY: the plugin exports this symbol with the signature the ABI names.
            let create: CreateFn = unsafe { std::mem::transmute(symbol) };
            // SAFETY: the document's bytes stay alive for the call, which is the contract.
            let pointer = unsafe { create(document.as_ptr(), document.len()) };
            if pointer.is_null() {
                return Err(self.last_error().unwrap_or_else(|| "the plugin refused".to_owned()));
            }
            // SAFETY: the plugin returns a leaked Box<Arc<dyn Renderer>>, and this is the
            // single place that takes it back. Both sides were built from one workspace.
            let boxed: Box<Arc<dyn Renderer>> = unsafe { Box::from_raw(pointer.cast()) };
            Ok(*boxed)
        }

        /// One exported symbol, or none.
        ///
        /// # Safety
        ///
        /// The plugin must be loaded, and the symbol must have the type the caller casts it
        /// to. Both are guaranteed by the ABI this module enforces.
        unsafe fn symbol(&self, name: &[u8]) -> Option<*mut c_void> {
            let name = CStr::from_bytes_with_nul(name).ok()?;
            let address = unsafe { GetProcAddress(self.module, name.as_ptr().cast()) }?;
            Some(address as *mut c_void)
        }
    }

    /// Puts a directory at the front of the process PATH, once.
    ///
    /// GStreamer opens plugins with `LOAD_WITH_ALTERED_SEARCH_PATH`, whose dependency search
    /// includes PATH but not the user directories AddDllDirectory records. A runtime that is
    /// not on PATH therefore loads its core but not its plugins.
    pub fn prepend_path(directory: &Path) {
        let Ok(current) = std::env::var("PATH") else {
            return;
        };
        let directory = directory.to_string_lossy();
        if current.split(';').any(|entry| entry.eq_ignore_ascii_case(directory.as_ref())) {
            return;
        }
        let joined = format!("{directory};{current}");
        // SAFETY: the variable is changed before a session or a plugin scan exists; the
        // plugin reads it when it loads, and no other thread of this program writes PATH.
        unsafe {
            std::env::set_var("PATH", joined);
        }
    }

    /// Adds a directory to the places a load searches for dependencies.
    pub fn add_dll_directory(directory: &Path) {
        if !directory.is_dir() {
            return;
        }
        let wide: Vec<u16> = directory.as_os_str().encode_wide().chain([0]).collect();
        // SAFETY: the path is a null-terminated UTF-16 string that outlives the call. The
        // cookie is deliberately dropped: a directory added for the process's life cannot
        // be removed with a handle that is no longer kept.
        unsafe {
            AddDllDirectory(wide.as_ptr());
        }
    }
}
