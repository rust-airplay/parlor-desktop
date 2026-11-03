# Parlor

Parlor 是一个基于 [rust-airplay](https://github.com/rust-airplay) 的桌面 AirPlay 接收器。

设备可以在 AirPlay 菜单中看到 Parlor，用 PIN 配对后投屏或播放音频。Parlor 是接收器外面的
那个程序：保存用户选择的设置、启动和停止接收器、显示 PIN 与已连接的设备，并在机器缺少播放
后端时提供安装。

协议与媒体处理不在这个仓库里。`rust-airplay` 负责应答设备并解密媒体流，播放后端负责解码
与呈现。Parlor 负责窗口、会话的生命周期，以及两个后端各自的依赖。

## 功能

- 接收 AirPlay 投屏与音频，支持一次性 PIN 或固定 PIN。
- 可通过设置切换 **FFmpeg**（两个可执行程序，构建时无原生依赖）与 **GStreamer**
  （运行时加载的插件）两种播放后端。
- 启动时若没有可用的播放后端会提示用户，并提供一键安装。
- 覆盖接收器的全部配置：名称、设备标识、端口、监听地址、广播、配对方式、密码、客户端策略、
  已记住的设备、广告的分辨率与帧率、特性位图、音频编码与延迟、音量曲线、视频节奏，以及全部
  诊断文件。
- 首页放置常用项：开始/停止、分辨率、帧率、后端。
- 支持浅色/深色主题（跟随系统），中英文切换。

界面中不使用任何 emoji。

## 构建与运行

需要 Windows 10 及以上、Rust 1.92+（MSVC 工具链）、Visual Studio 2022 Build Tools 的
C++ 桌面开发负载，以及 `PATH` 中的 CMake。GStreamer 后端还需要 GStreamer 的 MSVC 开发包
与 `pkg-config`。

```powershell
powershell -ExecutionPolicy Bypass -File scripts\setup-gstreamer.ps1   # 可选
powershell -ExecutionPolicy Bypass -File scripts\build.ps1 -Release
powershell -ExecutionPolicy Bypass -File scripts\run.ps1
```

没有 GStreamer 开发包时程序依然可以构建，GStreamer 后端会报告插件缺失。这是有意的：程序
必须能在没有 GStreamer 的机器上启动，才能提供一键安装。

详细说明见 [docs/BUILDING.md](docs/BUILDING.md) 与 [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)。

## 许可

GNU 通用公共许可证第 3 版或更高版本。接收器库同样以 GPL-3.0-or-later 发布。