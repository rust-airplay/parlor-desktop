# Parlor

[![CI](https://github.com/rust-airplay/parlor-desktop/actions/workflows/ci.yml/badge.svg)](https://github.com/rust-airplay/parlor-desktop/actions/workflows/ci.yml)
[![Release](https://github.com/rust-airplay/parlor-desktop/actions/workflows/release.yml/badge.svg)](https://github.com/rust-airplay/parlor-desktop/actions/workflows/release.yml)

<img src="assets/icon-128.png" width="80" align="right" alt="Parlor">

Parlor 是一个带桌面界面的 Windows AirPlay 接收器。设备可以在 AirPlay 菜单中看到 Parlor，
用 PIN 配对后投屏或播放音频。

它是接收器外面的那层窗口：保存用户选择的设置、启动和停止接收器、显示 PIN 与已连接的设备，
并负责查找或安装解码与呈现媒体流的程序。协议与媒体处理不在这个仓库里：
[`rust-airplay`](https://github.com/rust-airplay/rust-airplay) 负责应答设备并解密媒体流，
[`rust-airplay-gstream`](https://github.com/rust-airplay/rust-airplay-gstream) 与
[`rust-airplay-ffmpeg`](https://github.com/rust-airplay/rust-airplay-ffmpeg) 负责播放。

## 功能

- **屏幕镜像与音频**接收，支持一次性 PIN 或固定 PIN。
- **两种可切换的播放后端：**
  - **GStreamer**：通过运行时加载的插件播放，运行时带硬件解码器时自动使用。
  - **FFmpeg**：通过 `ffmpeg` 与 `ffplay` 播放，构建时没有原生依赖。
- **一键安装**缺失的后端到 `%LOCALAPPDATA%\Parlor\backends`，并记录环境变量，下次启动直接可用。
- **完整的接收器配置**：身份、名称、端口、监听地址、宣告、配对、密码、客户端策略、记住的设备、
  广播的显示尺寸与帧率、特性位图、音频编码与延迟、音量曲线、视频节奏，以及诊断文件。
- **主页只放观看时会用到的控件**：开始/停止、会话名称、镜像尺寸、帧率与后端。
- **浅色与深色主题**（默认跟随系统），**英文与简体中文**界面。
- **关于页的检测更新**：最新版本号记录在 [`Version`](Version)，有新版本时按钮打开 Release 页面。
- **发布只有一个可执行文件**：GStreamer 适配层内嵌在程序里，首次使用时释放到数据目录。

界面中不使用 emoji。

## 环境要求

- Windows 10 或更高版本，64 位。
- 构建需要 Rust 1.95 或更新版本，使用 MSVC 工具链。
- 构建不需要任何媒体库；GStreamer 与 FFmpeg 在运行时查找，两者都没有时可在应用内安装。

## 构建

```powershell
# 调试构建，同时构建 GStreamer 插件。
scripts\build.ps1

# 发布构建。
scripts\build.ps1 -Release
```

`scripts\run.ps1` 启动调试构建。工作区包含三个 crate：`parlor-desktop` 是程序本体，
`parlor-backend-api` 是插件边界，`parlor-gst-backend` 是 GStreamer 插件。

## 运行

启动 `parlor-desktop.exe`。机器上没有任何播放后端时，主页会提供一键安装；能安装 GStreamer 时
优先选它，因为它用 GPU 解码。

设备通过局域网连接：用界面上显示的 PIN 配对，然后在 AirPlay 菜单中投屏或播放音频。

## 播放后端

| | GStreamer | FFmpeg |
| --- | --- | --- |
| 交付方式 | 内嵌在程序中，首次使用时释放；运行时加载 GStreamer | `ffplay` / `ffmpeg` 程序 |
| 解码 | 可用时使用 GPU（`d3d11h264dec`、`d3d11h265dec`） | CPU，开启硬件解码后使用 GPU |
| 延迟 | 最低 | 较高，属于后备方案 |
| 安装 | 一键安装 | 一键安装 |

FFmpeg 的视频窗口默认是 1280x720，因为 `ffplay` 会按视频尺寸开窗。可在
**设置 > 播放 > FFmpeg > 视频输出** 中选 1600x900 或全屏。

## 值得注意的设置

- **镜像尺寸**：镜像是设备把自己的画面放大到广播尺寸，1920x1080 最清晰；更大的尺寸只会让
  设备端放大。FFmpeg 软件解码时上限为 1080p。
- **硬件解码**默认开启。FFmpeg 会询问解码器支持哪种加速方式，没有可用加速器时自动回退软件解码。
- **设置在接收器启动时生效**，修改后主页与侧栏会显示重启提示。
- 文件位于 `%LOCALAPPDATA%\Parlor`：`settings.toml`、身份、`logs\parlor.log` 与已安装的后端。

## 更新

最新版本的版本号记录在 [`Version`](Version)。**设置 > 关于 > 检测更新** 会读取该文件，
发现更新的版本时提供 Release 下载页。

新版本通过替换旧程序安装：发布包包含 `parlor-desktop.exe`、README、许可证与版本文件。
设置保存在 `%LOCALAPPDATA%\Parlor`，替换后仍然有效。

## 测试

```powershell
cargo test -p parlor-desktop -p parlor-backend-api
cargo clippy -p parlor-desktop -p parlor-backend-api --all-targets -- -D warnings
cargo fmt --all -- --check
```

两个渲染器测试默认被忽略，因为它们需要真实的后端：

```powershell
cargo build -p parlor-gst-backend
copy target\debug\parlor_gst_backend.dll target\debug\deps\
$env:PARLOR_GSTREAMER_DIR = "$PWD\.tools\gstreamer\runtime"
cargo test -p parlor-desktop --bin parlor-desktop -- --ignored --test-threads=1
```

## 发布

在 **Actions > Release > Run workflow** 中填写要创建的 tag（例如 `v0.2.0`），可选填写发布说明
与草稿标记。工作流会检查 tag 与 `Cargo.toml` 版本一致，在 `windows-latest` 上构建并测试，
打包程序与插件，创建 GitHub Release，然后把版本号写入 `main` 上的 `Version`，供检测更新读取。

## 文档

- [`docs/BUILDING.md`](docs/BUILDING.md)：工具链、图标与 GStreamer 加载器。
- [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md)：窗口、会话与后端如何协作。
- [`docs/feature-inventory.md`](docs/feature-inventory.md)：接收器的全部功能及其设置位置。
- [`AGENT.md`](AGENT.md)：本应用的构建需求。

## 许可证

GPL-3.0-or-later，见 [`LICENSE`](LICENSE)。
