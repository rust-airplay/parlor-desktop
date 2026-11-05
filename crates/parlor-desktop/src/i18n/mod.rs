// SPDX-License-Identifier: GPL-3.0-or-later
//! Every label the interface shows, in the languages it speaks.
//!
//! The text is one enum rather than one string table per page because a label that is
//! added to one language and not the other is a bug the compiler can catch: every
//! variant has both arms below, and a missing arm does not compile.

use crate::settings::LanguageChoice;

/// A language the interface is written in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lang {
    /// English.
    English,
    /// Simplified Chinese.
    Chinese,
}

impl Lang {
    /// The language to start in.
    ///
    /// A person who chose one gets it. A person who did not gets the operating system's
    /// language when that is Chinese, and English otherwise, because an English label is
    /// readable to more people than a Chinese one is.
    pub fn resolve(choice: LanguageChoice) -> Self {
        match choice {
            LanguageChoice::English => Self::English,
            LanguageChoice::Chinese => Self::Chinese,
            LanguageChoice::System => {
                if system_language_is_chinese() {
                    Self::Chinese
                } else {
                    Self::English
                }
            }
        }
    }
}

/// The operating system's locale name, such as `zh-CN`.
///
/// This is the locale, not the display language: a machine can be set to `zh-CN` and still
/// show its own interface in English. It is the second answer, and it is also what the log
/// reports when a person asks why the window is in the language it is in.
#[allow(unsafe_code)]
pub fn system_locale_name() -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        let mut buffer = [0u16; 85];
        // SAFETY: the buffer is valid for the length passed, and the call writes at most
        // that many UTF-16 code units and a terminator.
        let written = unsafe {
            windows_sys::Win32::Globalization::GetUserDefaultLocaleName(
                buffer.as_mut_ptr(),
                i32::try_from(buffer.len()).unwrap_or(85),
            )
        };
        if written <= 1 {
            return None;
        }
        let units = &buffer[..usize::try_from(written).unwrap_or(0) - 1];
        Some(String::from_utf16_lossy(units))
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::env::var("LANG").ok()
    }
}

/// The Windows UI language identifier, which is the language Windows itself speaks.
#[allow(unsafe_code)]
fn windows_ui_language() -> Option<u16> {
    #[cfg(target_os = "windows")]
    {
        // SAFETY: the call takes no arguments and returns a LANGID.
        Some(unsafe { windows_sys::Win32::Globalization::GetUserDefaultUILanguage() })
    }
    #[cfg(not(target_os = "windows"))]
    {
        None
    }
}

/// Whether the operating system's language is Chinese.
///
/// The UI language is asked first, because that is the language a person selected in
/// Windows and the one the system's own labels are written in. The locale is the second
/// answer, because a machine can run an English Windows set to a Chinese locale, and the
/// window should follow the language its user reads.
fn system_language_is_chinese() -> bool {
    if windows_ui_language().is_some_and(|language| language & 0x3ff == 0x04) {
        return true;
    }
    system_locale_name().is_some_and(|locale| locale.to_lowercase().starts_with("zh"))
}

/// One line about the language this program resolved, for the log.
pub fn describe_system_language() -> String {
    let locale = system_locale_name().unwrap_or_else(|| "unknown".to_owned());
    let ui = windows_ui_language().map_or_else(|| "none".to_owned(), |id| format!("0x{id:04x}"));
    let resolved = match Lang::resolve(LanguageChoice::System) {
        Lang::English => "English",
        Lang::Chinese => "Chinese",
    };
    format!("locale {locale}, UI language {ui}, system language resolves to {resolved}")
}

/// One label or sentence the interface shows.
///
/// The variants are grouped the way the pages are, so a page's labels are found together.
/// The table is deliberately wider than the pages that use it: a label belongs to an
/// idea, and a page that grows into the idea should find its words already written and
/// already translated. That is also why this is one enum and not two files.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Text {
    // The window, and the navigation.
    NavHome,
    NavSettings,
    NavAbout,
    StatusStopped,
    StatusStarting,
    StatusRunning,
    StatusStopping,
    StatusFailed,
    Version,

    // The home page.
    HomeTitle,
    HomeSubtitle,
    Start,
    StartCasting,
    Stop,
    CheckUpdates,
    UpdateChecking,
    UpdateUpToDate,
    UpdateAvailable,
    UpdateFailed,
    Download,
    ReceiverName,
    ControlPort,
    Resolution,
    CustomSize,
    Width,
    Height,
    FrameRate,
    RefreshRate,
    Backend,
    NoBackendTitle,
    NoBackendBody,
    Install,
    ManageBackends,
    PinTitle,
    PinHint,
    PendingTitle,
    PendingBody,
    Restart,
    Devices,
    NoDevices,
    NowPlaying,
    Activity,
    QualityTitle,
    SessionTitle,
    Connected,
    Waiting,
    LastError,

    // Settings, by page.
    SettingsGeneral,
    SettingsAppearance,
    SettingsNetwork,
    SettingsSecurity,
    SettingsVideo,
    SettingsAudio,
    SettingsBackend,
    SettingsDiagnostics,
    SettingsAbout,
    SettingsDescription,

    // General.
    DeviceId,
    DeviceIdHint,
    PaceVideo,
    PaceVideoHint,
    NoHold,
    NoHoldHint,
    FeedbackTimeout,
    FeedbackTimeoutHint,
    Seconds,
    RestartHint,

    // Appearance.
    Theme,
    ThemeSystem,
    ThemeLight,
    ThemeDark,
    Language,
    LangSystem,
    LangEnglish,
    LangChinese,

    // Network.
    BindAddress,
    Advertise,
    AdvertiseHint,
    MediaPorts,
    PortsAutomatic,
    PortsLegacy,
    PortsCustom,
    TimingPort,
    UdpControlPort,
    DataPort,
    MirrorPort,

    // Security.
    Pairing,
    PinOneTime,
    PinFixed,
    PinDisabled,
    PinDigits,
    Password,
    PasswordHint,
    RestrictClients,
    RestrictClientsHint,
    AllowedDevices,
    BlockedDevices,
    OnePerLine,
    KnownDevices,
    Forget,
    ForgetAll,
    RememberClients,
    RememberClientsHint,
    RegenerateIdentity,
    RegenerateIdentityHint,
    Confirm,
    Cancel,
    Save,
    Saved,
    Error,
    Close,

    // Video.
    Hevc,
    HevcHint,
    Hls,
    HlsHint,
    Slideshow,
    SlideshowHint,
    Overscanned,
    OverscannedHint,
    ResolutionHint,
    FrameRateHint,
    BackendHint,
    HardwareDecoding,
    HardwareDecodingHint,

    // Audio.
    AudioCodec,
    CodecAlac,
    CodecAacEld,
    Latency,
    LatencyHint,
    InitialVolume,
    Taper,
    TaperHint,
    LowDb,
    HighDb,
    Decibels,

    // Backend.
    BackendKind,
    GStreamer,
    Ffmpeg,
    Available,
    NotInstalled,
    NotInstalledHint,
    AutoChoice,
    CustomEntry,
    BindAllInterfaces,
    BindThisMachine,
    BindLocalNetwork,
    Broken,
    InstallNow,
    Reinstall,
    Uninstall,
    Location,
    AudioSink,
    VideoSink,
    Decoder,
    Player,
    H264Chain,
    H265Chain,
    H264Parser,
    H264Decoder,
    H265Parser,
    H265Decoder,
    Parser,
    DecoderElement,
    WarmVideo,
    WarmVideoHint,
    WarmCodec,
    Pacing,
    PacingHint,
    EmptyUsesBackendDefault,
    PluginMissing,
    PluginMissingHint,
    BackendSearchHint,
    OpenApplicationFolder,

    // Diagnostics.
    LogLevel,
    LevelOff,
    LevelError,
    LevelWarn,
    LevelInfo,
    LevelDebug,
    LevelTrace,
    TraceLog,
    TraceLogHint,
    DumpVideo,
    DumpVideoHint,
    DumpAudio,
    DumpAudioHint,
    StreamingReports,
    StreamingReportsHint,
    SetupBody,
    SetupBodyHint,
    OpenLogs,
    Log,
    ClearLog,
    NoLog,

    // About.
    AboutTitle,
    AboutBody,
    License,
    LicenseBody,
    Homepage,
    Documentation,

    // Installation.
    InstallTitle,
    InstallPreparing,
    InstallDownloading,
    InstallExtracting,
    InstallVerifying,
    InstallDone,
    InstallFailed,
    InstallCancel,
    InstallRestart,

    // The first-run dialog.
    NoBackendDialogTitle,
    NoBackendDialogBody,
    ContinueWithout,
    Quit,
    InstallFfmpeg,
    InstallGStreamer,

    // Confirmation dialogs.
    ConfirmRegenerateBody,
    ConfirmForgetAllBody,
}

impl Text {
    /// The label in this language.
    pub fn of(self, lang: Lang) -> &'static str {
        match lang {
            Lang::English => self.english(),
            Lang::Chinese => self.chinese(),
        }
    }

    /// The English label.
    fn english(self) -> &'static str {
        match self {
            Self::NavHome => "Home",
            Self::NavSettings => "Settings",
            Self::NavAbout => "About",
            Self::StatusStopped => "Stopped",
            Self::StatusStarting => "Starting",
            Self::StatusRunning => "Running",
            Self::StatusStopping => "Stopping",
            Self::StatusFailed => "Failed",
            Self::Version => "Version",
            Self::HomeTitle => "Screen mirroring",
            Self::HomeSubtitle => {
                "Start the receiver, then choose Parlor in the AirPlay menu of your device."
            }
            Self::Start => "Start",
            Self::StartCasting => "Start casting",
            Self::Stop => "Stop",
            Self::CheckUpdates => "Check for updates",
            Self::UpdateChecking => "Checking for updates...",
            Self::UpdateUpToDate => "This is the latest version.",
            Self::UpdateAvailable => "New version",
            Self::UpdateFailed => "Could not check for updates",
            Self::Download => "Download",
            Self::ReceiverName => "Name",
            Self::ControlPort => "Control port",
            Self::Resolution => "Resolution",
            Self::CustomSize => "Custom",
            Self::Width => "Width",
            Self::Height => "Height",
            Self::FrameRate => "Frame rate",
            Self::RefreshRate => "Refresh rate",
            Self::Backend => "Backend",
            Self::NoBackendTitle => "No playback backend",
            Self::NoBackendBody => {
                "Parlor plays what a device sends through GStreamer or FFmpeg. Neither was found."
            }
            Self::Install => "Install",
            Self::ManageBackends => "Manage backends",
            Self::PendingTitle => "Changes are not in effect",
            Self::PendingBody => "Restart the receiver to apply them.",
            Self::Restart => "Restart",
            Self::PinTitle => "Enter this PIN on your device",
            Self::PinHint => "The PIN is shown only while a device is pairing.",
            Self::Devices => "Devices",
            Self::NoDevices => "No device has connected yet.",
            Self::NowPlaying => "Now playing",
            Self::Activity => "Activity",
            Self::QualityTitle => "Picture",
            Self::SessionTitle => "Session",
            Self::Connected => "Connected",
            Self::Waiting => "Waiting for a device",
            Self::LastError => "Last error",
            Self::SettingsGeneral => "General",
            Self::SettingsAppearance => "Appearance",
            Self::SettingsNetwork => "Network",
            Self::SettingsSecurity => "Security",
            Self::SettingsVideo => "Video",
            Self::SettingsAudio => "Audio",
            Self::SettingsBackend => "Playback",
            Self::SettingsDiagnostics => "Diagnostics",
            Self::SettingsAbout => "About Parlor",
            Self::SettingsDescription => "Changes apply the next time the receiver starts.",
            Self::DeviceId => "Device identifier",
            Self::DeviceIdHint => {
                "The address a device remembers. Empty means one derived from the identity."
            }
            Self::PaceVideo => "Pace video",
            Self::PaceVideoHint => {
                "Hold frames back to the spacing they were captured at. Off shows the newest                  picture as soon as it arrives."
            }
            Self::NoHold => "One connection per device",
            Self::NoHoldHint => "A new connection from the same device drops the previous one.",
            Self::FeedbackTimeout => "Close silent sessions after",
            Self::FeedbackTimeoutHint => "Empty keeps a session whose device stopped reporting.",
            Self::Seconds => "seconds",
            Self::RestartHint => "Takes effect the next time the receiver starts.",
            Self::Theme => "Theme",
            Self::ThemeSystem => "Follow system",
            Self::ThemeLight => "Light",
            Self::ThemeDark => "Dark",
            Self::Language => "Language",
            Self::LangSystem => "Follow system",
            Self::LangEnglish => "English",
            Self::LangChinese => "简体中文",
            Self::BindAddress => "Listen on",
            Self::Advertise => "Announce this receiver",
            Self::AdvertiseHint => {
                "Off keeps the receiver off the network's device list; a device must be told                  the address instead."
            }
            Self::MediaPorts => "Media ports",
            Self::PortsAutomatic => "Automatic",
            Self::PortsLegacy => "Protocol layout",
            Self::PortsCustom => "Custom",
            Self::TimingPort => "Timing port",
            Self::UdpControlPort => "Control port (UDP)",
            Self::DataPort => "Data port",
            Self::MirrorPort => "Mirror port",
            Self::Pairing => "Pairing",
            Self::PinOneTime => "One-time PIN",
            Self::PinFixed => "Fixed PIN",
            Self::PinDisabled => "No PIN",
            Self::PinDigits => "PIN",
            Self::Password => "Password",
            Self::PasswordHint => "Optional. A device must give it before it may start a session.",
            Self::RestrictClients => "Only allowed devices",
            Self::RestrictClientsHint => "Refuse every device that is not in the allowed list.",
            Self::AllowedDevices => "Allowed devices",
            Self::BlockedDevices => "Blocked devices",
            Self::OnePerLine => "One device identifier per line.",
            Self::KnownDevices => "Known devices",
            Self::Forget => "Forget",
            Self::ForgetAll => "Forget all",
            Self::RememberClients => "Remember paired devices",
            Self::RememberClientsHint => "A remembered device reconnects without pairing again.",
            Self::RegenerateIdentity => "New identity",
            Self::RegenerateIdentityHint => {
                "Every device sees a new receiver and has to pair again."
            }
            Self::Confirm => "Confirm",
            Self::Cancel => "Cancel",
            Self::Save => "Save",
            Self::Saved => "Saved",
            Self::Error => "Error",
            Self::Close => "Close",
            Self::Hevc => "H.265 mirroring",
            Self::HevcHint => {
                "Offer HEVC to devices that support it. It sends roughly half the bytes."
            }
            Self::Hls => "HTTP Live Streaming",
            Self::HlsHint => "Advertised to devices only; the receiver does not serve HLS.",
            Self::Slideshow => "Slideshow",
            Self::SlideshowHint => "Advertised to devices only; the receiver has no slideshow.",
            Self::Overscanned => "Overscanned picture",
            Self::OverscannedHint => "Tells the device the picture is cropped at the edges.",
            Self::ResolutionHint => {
                "A mirror is the device's own picture scaled to this size. 1920x1080 stays sharp; larger sizes only ask the device to upscale, which looks softer and costs more. The FFmpeg backend stops at 1080p."
            }
            Self::FrameRateHint => {
                "Mirror clients encode at most 60 frames a second. A higher value only adds load."
            }
            Self::BackendHint => {
                "GStreamer decodes video on the GPU and is the one to use for a mirror. The FFmpeg backend plays through ffplay, which decodes on the CPU unless hardware decoding is switched on: a large mirror stutters, and a stream that starts mid-picture shows green until the next key frame."
            }
            Self::HardwareDecoding => "Hardware decoding",
            Self::HardwareDecodingHint => {
                "Asks the player for the GPU decoder its build lists, and falls back to software when it has none. It buys CPU rather than latency: a frame decoded on the GPU comes back to memory to be shown."
            }
            Self::AudioCodec => "Audio codec",
            Self::CodecAlac => "Apple Lossless",
            Self::CodecAacEld => "AAC-ELD",
            Self::Latency => "Audio latency",
            Self::LatencyHint => {
                "How far ahead of playback the device should send. Too small is late, too                  large starts late."
            }
            Self::InitialVolume => "Start volume",
            Self::Taper => "Tapered volume",
            Self::TaperHint => "Map the device's slider logarithmically instead of linearly.",
            Self::LowDb => "Bottom of the slider",
            Self::HighDb => "Top of the slider",
            Self::Decibels => "dB",
            Self::BackendKind => "Playback backend",
            Self::GStreamer => "GStreamer",
            Self::Ffmpeg => "FFmpeg",
            Self::Available => "Available",
            Self::NotInstalled => "Not installed",
            Self::NotInstalledHint => "Install it to play through this backend.",
            Self::AutoChoice => "Automatic",
            Self::CustomEntry => "Custom",
            Self::BindAllInterfaces => "All interfaces",
            Self::BindThisMachine => "This machine only",
            Self::BindLocalNetwork => "Local network",
            Self::Broken => "Cannot be used",
            Self::InstallNow => "Install",
            Self::Reinstall => "Install again",
            Self::Uninstall => "Remove",
            Self::Location => "Location",
            Self::AudioSink => "Audio sink",
            Self::VideoSink => "Video sink",
            Self::Decoder => "ffmpeg",
            Self::Player => "Player",
            Self::H264Chain => "H.264",
            Self::H265Chain => "H.265",
            Self::H264Parser => "H.264 parser",
            Self::H264Decoder => "H.264 decoder",
            Self::H265Parser => "H.265 parser",
            Self::H265Decoder => "H.265 decoder",
            Self::Parser => "Parser",
            Self::DecoderElement => "Decoder",
            Self::WarmVideo => "Keep the video window open",
            Self::WarmVideoHint => "A window that already exists shows the first frame sooner.",
            Self::WarmCodec => "Started for",
            Self::Pacing => "Pace video in the backend",
            Self::PacingHint => {
                "Relevant only to a caller driving the stream directly; the receiver paces                  video itself."
            }
            Self::EmptyUsesBackendDefault => "Empty uses the backend's choice.",
            Self::BackendSearchHint => {
                "A backend is looked for in the lib folder beside the application first, then in \
                 PARLOR_GSTREAMER_DIR or PARLOR_FFMPEG_DIR, then on the PATH."
            }
            Self::OpenApplicationFolder => "Open the application folder",
            Self::PluginMissing => "The GStreamer plugin is missing",
            Self::PluginMissingHint => {
                "This build has no GStreamer plugin beside it. Install the GStreamer runtime                  and rebuild with the plugin."
            }
            Self::LogLevel => "Log level",
            Self::LevelOff => "Off",
            Self::LevelError => "Errors",
            Self::LevelWarn => "Warnings",
            Self::LevelInfo => "Milestones",
            Self::LevelDebug => "Requests",
            Self::LevelTrace => "Everything",
            Self::TraceLog => "Request trace file",
            Self::TraceLogHint => "Appends every request and its answer to a file.",
            Self::DumpVideo => "Write the first video frames",
            Self::DumpVideoHint => "A file a picture can be diagnosed from.",
            Self::DumpAudio => "Write the audio payloads",
            Self::DumpAudioHint => "A file an audio stream can be diagnosed from.",
            Self::StreamingReports => "Log the device's reports",
            Self::StreamingReportsHint => "What the device itself thinks of the stream.",
            Self::SetupBody => "Report SETUP bodies",
            Self::SetupBodyHint => "Includes the session key. Use it only for a diagnosis.",
            Self::OpenLogs => "Open the log folder",
            Self::Log => "Log",
            Self::ClearLog => "Clear",
            Self::NoLog => "Nothing recorded yet.",
            Self::AboutTitle => "Parlor",
            Self::AboutBody => {
                "An AirPlay receiver for the desktop, built on the rust-airplay libraries."
            }
            Self::License => "License",
            Self::LicenseBody => "GPL-3.0-or-later",
            Self::Homepage => "Project",
            Self::Documentation => "Documentation",
            Self::InstallTitle => "Install a backend",
            Self::InstallPreparing => "Preparing",
            Self::InstallDownloading => "Downloading",
            Self::InstallExtracting => "Extracting",
            Self::InstallVerifying => "Checking",
            Self::InstallDone => "Installed",
            Self::InstallFailed => "The installation failed",
            Self::InstallCancel => "Cancel",
            Self::InstallRestart => "Start a new session to use it.",
            Self::NoBackendDialogTitle => "No playback backend found",
            Self::NoBackendDialogBody => {
                "Parlor can receive a device now, but nothing would play. Install a backend,                  or continue and choose one later in Settings."
            }
            Self::ContinueWithout => "Continue without playback",
            Self::Quit => "Quit",
            Self::InstallFfmpeg => "Install FFmpeg",
            Self::InstallGStreamer => "Install GStreamer",
            Self::ConfirmRegenerateBody => {
                "Every device will see a new receiver and will have to pair again."
            }
            Self::ConfirmForgetAllBody => "Every remembered device will have to pair again.",
        }
    }

    /// The Simplified Chinese label.
    fn chinese(self) -> &'static str {
        match self {
            Self::NavHome => "主页",
            Self::NavSettings => "设置",
            Self::NavAbout => "关于",
            Self::StatusStopped => "已停止",
            Self::StatusStarting => "正在启动",
            Self::StatusRunning => "运行中",
            Self::StatusStopping => "正在停止",
            Self::StatusFailed => "失败",
            Self::Version => "版本",
            Self::HomeTitle => "屏幕镜像",
            Self::HomeSubtitle => "启动接收器，然后在设备的 AirPlay 菜单中选择 Parlor。",
            Self::Start => "开始",
            Self::StartCasting => "开启投屏",
            Self::Stop => "停止",
            Self::CheckUpdates => "检测更新",
            Self::UpdateChecking => "正在检查更新…",
            Self::UpdateUpToDate => "已是最新版本。",
            Self::UpdateAvailable => "新版本",
            Self::UpdateFailed => "检查更新失败",
            Self::Download => "下载",
            Self::ReceiverName => "名称",
            Self::ControlPort => "控制端口",
            Self::Resolution => "分辨率",
            Self::CustomSize => "自定义",
            Self::Width => "宽度",
            Self::Height => "高度",
            Self::FrameRate => "帧率",
            Self::RefreshRate => "刷新率",
            Self::Backend => "播放后端",
            Self::NoBackendTitle => "没有可用的播放后端",
            Self::NoBackendBody => {
                "Parlor 通过 GStreamer 或 FFmpeg 播放设备投送的内容，但两者都没有找到。"
            }
            Self::Install => "安装",
            Self::ManageBackends => "管理后端",
            Self::PendingTitle => "更改尚未生效",
            Self::PendingBody => "重启接收器后应用这些更改。",
            Self::Restart => "重启",
            Self::PinTitle => "请在设备上输入此 PIN",
            Self::PinHint => "只有在设备配对时才会显示 PIN。",
            Self::Devices => "设备",
            Self::NoDevices => "还没有设备连接过。",
            Self::NowPlaying => "正在播放",
            Self::Activity => "活动",
            Self::QualityTitle => "画面",
            Self::SessionTitle => "会话",
            Self::Connected => "已连接",
            Self::Waiting => "等待设备连接",
            Self::LastError => "最近的错误",
            Self::SettingsGeneral => "通用",
            Self::SettingsAppearance => "外观",
            Self::SettingsNetwork => "网络",
            Self::SettingsSecurity => "安全",
            Self::SettingsVideo => "视频",
            Self::SettingsAudio => "音频",
            Self::SettingsBackend => "播放",
            Self::SettingsDiagnostics => "诊断",
            Self::SettingsAbout => "关于 Parlor",
            Self::SettingsDescription => "更改会在接收器下次启动时生效。",
            Self::DeviceId => "设备标识",
            Self::DeviceIdHint => "设备记住的地址。留空表示由身份种子派生。",
            Self::PaceVideo => "平滑视频",
            Self::PaceVideoHint => "按采集间隔均匀输出画面。关闭时总是尽快显示最新画面。",
            Self::NoHold => "每台设备只保留一个连接",
            Self::NoHoldHint => "同一设备的新连接会断开之前的连接。",
            Self::FeedbackTimeout => "关闭静默会话的超时",
            Self::FeedbackTimeoutHint => "留空表示设备停止汇报后仍保持连接。",
            Self::Seconds => "秒",
            Self::RestartHint => "在接收器下次启动时生效。",
            Self::Theme => "主题",
            Self::ThemeSystem => "跟随系统",
            Self::ThemeLight => "浅色",
            Self::ThemeDark => "深色",
            Self::Language => "语言",
            Self::LangSystem => "跟随系统",
            Self::LangEnglish => "English",
            Self::LangChinese => "简体中文",
            Self::BindAddress => "监听地址",
            Self::Advertise => "在局域网中广播此接收器",
            Self::AdvertiseHint => "关闭后设备列表中不会出现此接收器，需要手动输入地址。",
            Self::MediaPorts => "媒体端口",
            Self::PortsAutomatic => "自动",
            Self::PortsLegacy => "协议默认布局",
            Self::PortsCustom => "自定义",
            Self::TimingPort => "授时端口",
            Self::UdpControlPort => "控制端口（UDP）",
            Self::DataPort => "数据端口",
            Self::MirrorPort => "镜像端口",
            Self::Pairing => "配对方式",
            Self::PinOneTime => "一次性 PIN",
            Self::PinFixed => "固定 PIN",
            Self::PinDisabled => "无 PIN",
            Self::PinDigits => "PIN",
            Self::Password => "密码",
            Self::PasswordHint => "可选。设备开始会话前必须提供密码。",
            Self::RestrictClients => "仅允许列表中的设备",
            Self::RestrictClientsHint => "拒绝所有不在允许列表中的设备。",
            Self::AllowedDevices => "允许的设备",
            Self::BlockedDevices => "拒绝的设备",
            Self::OnePerLine => "每行一个设备标识。",
            Self::KnownDevices => "已知设备",
            Self::Forget => "忘记",
            Self::ForgetAll => "全部忘记",
            Self::RememberClients => "记住已配对的设备",
            Self::RememberClientsHint => "已记住的设备无需再次配对即可连接。",
            Self::RegenerateIdentity => "更换身份",
            Self::RegenerateIdentityHint => "所有设备都会看到一个全新的接收器，需要重新配对。",
            Self::Confirm => "确认",
            Self::Cancel => "取消",
            Self::Save => "保存",
            Self::Saved => "已保存",
            Self::Error => "错误",
            Self::Close => "关闭",
            Self::Hevc => "H.265 镜像",
            Self::HevcHint => "向支持 HEVC 的设备提供该编码，通常只有一半的数据量。",
            Self::Hls => "HTTP Live Streaming",
            Self::HlsHint => "仅向设备声明，接收器并不提供 HLS。",
            Self::Slideshow => "幻灯片",
            Self::SlideshowHint => "仅向设备声明，接收器没有幻灯片功能。",
            Self::Overscanned => "过扫描画面",
            Self::OverscannedHint => "告诉设备画面边缘会被裁掉。",
            Self::ResolutionHint => {
                "镜像是把设备画面放大到这个尺寸。1920x1080 最清晰；更大的尺寸只会让设备端放大，画面变软、带宽更高。FFmpeg 后端上限 1080p。"
            }
            Self::FrameRateHint => "镜像客户端通常最多 60 帧。设得更高只会增加负载。",
            Self::BackendHint => {
                "GStreamer 用 GPU 解码视频，镜像播放应选它。FFmpeg 后端通过 ffplay 解码，默认用 CPU，可在下方开启硬件解码：大画幅会卡顿，若从半途开始接收，关键帧到达前会显示绿屏。"
            }
            Self::HardwareDecoding => "硬件解码",
            Self::HardwareDecodingHint => {
                "让播放器使用它支持的 GPU 解码器；没有可用解码器时自动回到软件解码。它节省的是 CPU 而不是延迟：GPU 解出的帧仍要回到内存才能显示。"
            }
            Self::AudioCodec => "音频编码",
            Self::CodecAlac => "Apple Lossless",
            Self::CodecAacEld => "AAC-ELD",
            Self::Latency => "音频延迟",
            Self::LatencyHint => "设备应提前多少时间发送音频。太小会迟到，太大则开始得晚。",
            Self::InitialVolume => "初始音量",
            Self::Taper => "对数音量",
            Self::TaperHint => "按对数而非线性映射设备的音量滑块。",
            Self::LowDb => "滑块底部的衰减",
            Self::HighDb => "滑块顶部的衰减",
            Self::Decibels => "dB",
            Self::BackendKind => "播放后端",
            Self::GStreamer => "GStreamer",
            Self::Ffmpeg => "FFmpeg",
            Self::Available => "可用",
            Self::NotInstalled => "未安装",
            Self::NotInstalledHint => "安装后即可用此后端播放。",
            Self::AutoChoice => "自动",
            Self::CustomEntry => "自定义",
            Self::BindAllInterfaces => "所有网卡",
            Self::BindThisMachine => "仅本机",
            Self::BindLocalNetwork => "本机局域网",
            Self::Broken => "无法使用",
            Self::InstallNow => "安装",
            Self::Reinstall => "重新安装",
            Self::Uninstall => "移除",
            Self::Location => "位置",
            Self::AudioSink => "音频输出",
            Self::VideoSink => "视频输出",
            Self::Decoder => "ffmpeg",
            Self::Player => "播放器",
            Self::H264Chain => "H.264",
            Self::H265Chain => "H.265",
            Self::H264Parser => "H.264 解析器",
            Self::H264Decoder => "H.264 解码器",
            Self::H265Parser => "H.265 解析器",
            Self::H265Decoder => "H.265 解码器",
            Self::Parser => "解析器",
            Self::DecoderElement => "解码器",
            Self::WarmVideo => "保持视频窗口打开",
            Self::WarmVideoHint => "提前打开的窗口能更快显示第一帧。",
            Self::WarmCodec => "预启动编码",
            Self::Pacing => "由后端控制视频节奏",
            Self::PacingHint => "仅对直接驱动视频流的调用者有意义，接收器自己会控制节奏。",
            Self::EmptyUsesBackendDefault => "留空表示使用后端的自动选择。",
            Self::BackendSearchHint => {
                "后端会先在与程序同级的 lib 目录中查找，其次查找 PARLOR_GSTREAMER_DIR 或 \
                 PARLOR_FFMPEG_DIR，最后才查找 PATH。"
            }
            Self::OpenApplicationFolder => "打开程序所在目录",
            Self::PluginMissing => "缺少 GStreamer 插件",
            Self::PluginMissingHint => {
                "此版本旁边没有 GStreamer 插件。请安装 GStreamer 运行时并重新构建插件。"
            }
            Self::LogLevel => "日志级别",
            Self::LevelOff => "关闭",
            Self::LevelError => "错误",
            Self::LevelWarn => "警告",
            Self::LevelInfo => "关键事件",
            Self::LevelDebug => "请求",
            Self::LevelTrace => "全部",
            Self::TraceLog => "请求跟踪文件",
            Self::TraceLogHint => "把每个请求及其应答追加到文件。",
            Self::DumpVideo => "写出首批视频帧",
            Self::DumpVideoHint => "用于诊断画面问题的文件。",
            Self::DumpAudio => "写出音频负载",
            Self::DumpAudioHint => "用于诊断音频流的文件。",
            Self::StreamingReports => "记录设备自己的汇报",
            Self::StreamingReportsHint => "设备对视频流的自我评估。",
            Self::SetupBody => "记录 SETUP 请求体",
            Self::SetupBodyHint => "其中包含会话密钥，仅在诊断时开启。",
            Self::OpenLogs => "打开日志目录",
            Self::Log => "日志",
            Self::ClearLog => "清空",
            Self::NoLog => "还没有记录。",
            Self::AboutTitle => "Parlor",
            Self::AboutBody => "基于 rust-airplay 系列库构建的桌面 AirPlay 接收器。",
            Self::License => "许可证",
            Self::LicenseBody => "GPL-3.0-or-later",
            Self::Homepage => "项目主页",
            Self::Documentation => "文档",
            Self::InstallTitle => "安装后端",
            Self::InstallPreparing => "准备中",
            Self::InstallDownloading => "下载中",
            Self::InstallExtracting => "解压中",
            Self::InstallVerifying => "校验中",
            Self::InstallDone => "已安装",
            Self::InstallFailed => "安装失败",
            Self::InstallCancel => "取消",
            Self::InstallRestart => "开始新的会话即可使用。",
            Self::NoBackendDialogTitle => "没有找到播放后端",
            Self::NoBackendDialogBody => {
                "Parlor 现在可以接收设备，但没有任何东西能播放。请安装一个后端，或者继续并在设置中选择。"
            }
            Self::ContinueWithout => "不安装，继续",
            Self::Quit => "退出",
            Self::InstallFfmpeg => "安装 FFmpeg",
            Self::InstallGStreamer => "安装 GStreamer",
            Self::ConfirmRegenerateBody => "所有设备都会看到一个全新的接收器，需要重新配对。",
            Self::ConfirmForgetAllBody => "所有已记住的设备都需要重新配对。",
        }
    }
}
