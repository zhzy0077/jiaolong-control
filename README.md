# 蛟龙控制中心（Rust / GPUI）

Linux 风扇控制 MVP，复用“蛟龙游戏控制中心”原程序的 MiInterface 固件协议。

提供 CPU/GPU 转速显示、最大风扇开关、100 RPM 步进调节、各模式范围与原格式设定保存。转速与模式每 10 秒刷新，也可点击“刷新”。普通模式按当前固件模式选择范围；已开启的自定义模式使用狂暴模式范围。检测到模式切换后保存离开模式的风扇设定，并恢复进入模式的设定。

| 模式 | 风扇设定范围 | 默认值 |
| --- | --- | --- |
| 狂暴 | 5000–5800 RPM | 5300 RPM |
| 游戏 | 3500–5000 RPM | 4300 RPM |
| 办公 | 2200–3500 RPM | 2700 RPM |

风扇控制页的“运行模式”可切换办公 / 游戏 / 狂暴，通过原程序的 WMI 命令 8 写入固件并读回确认，保存当前模式风扇设定并恢复目标模式设定。选择普通模式会关闭已开启的自定义模式；Type-C 供电下按原程序规则拒绝切换。

风扇设定滑块使用固定的 2200–5800 RPM 刻度，当前模式允许的范围以颜色突出，区间外轨道置灰。拖动时预览目标转速，松开后按 100 RPM 步长写入固件；加减按钮保留精确步进。滑块支持方向键调节和 Home / End 跳到当前模式的最低 / 最高转速，Escape 取消拖动。

## 界面与应用设置

界面按 `design/jiaolong-fan-control.op` 的 Adwaita mock 实现：侧栏、CPU/GPU 转速卡片、风扇设定、最大风扇开关和刷新栏。设备概览、性能调节、键盘与灯光、设备设置具有标记为“预留”的功能扩展页面。

侧栏底部的“应用设置”提供系统 / 浅色 / 深色外观，以及简体中文 / English 界面语言。系统选项通过 GPUI 的桌面外观通知跟随变化；显式选择浅色或深色时使用所选主题。更改即时生效并保存；读写失败时在设置页显示错误，继续使用当前偏好。

应用偏好存放在 `$XDG_CONFIG_HOME/jiaolong-control/preferences.json`，默认 `~/.config/jiaolong-control/preferences.json`；Windows 使用 `%APPDATA%/jiaolong-control/preferences.json`。该文件与原格式的 `AppMachineInfo` 分开。

```json
{ "theme": "system", "language": "zh_cn" }
```

`theme` 可选 `system`、`light`、`dark`；`language` 可选 `zh_cn`、`en`。支持 Tab / Shift+Tab 焦点导航、Enter / Space 激活控件、Escape 关闭语言菜单。

界面使用系统 fontconfig 的默认无衬线字体，字形回退由系统字体库处理。

仅预览界面、使用示例数据：

```bash
./dist/jiaolong-control --preview-ui=light,settings,en
./dist/jiaolong-control --preview-ui=dark
```

预览参数可组合 `light` / `dark`、`settings` / `features` / `office` / `error`、`en`、`menu`；预览使用内存中的示例风扇状态与应用偏好。

## 运行

适用硬件的 WMI GUID：`B60BFB48-3E5B-49E4-A0E9-8CFFE1B3434B`。需要与当前内核匹配的开发头文件、make、gcc；图形环境支持 Wayland 或 X11。

```bash
./scripts/install-driver.sh
./dist/jiaolong-control
```

安装脚本构建当前内核模块、配置开机加载与桌面会话的 udev 访问权限。内核更新后重新运行安装脚本。普通用户运行图形程序。配置文件 `AppMachineInfo` 位于程序旁边，保持原程序的 UTF-8/Base64 九行格式；可将原文件复制到该位置使用。

卸载驱动：关闭程序后运行 `./scripts/uninstall-driver.sh`。

## 构建和验证

```bash
cargo build --release --locked
cargo test --locked
make -C linux
```

Fedora 图形链接依赖：`libxkbcommon-devel libxkbcommon-x11-devel wayland-devel fontconfig-devel`。Rust 依赖由 `Cargo.lock` 固定。GPUI、gpui_platform 与 gpui_wgpu 使用包含 [Zed PR #62832](https://github.com/zed-industries/zed/pull/62832) 的固定提交 `522b0e2ac290da1f6e101f09eae3bb6958940b15`，修复可变字体的字重绘制；平台初始化使用 `gpui_platform::application()`。

## 固件协议与测试

请求保持 32 字节：索引 1 为读取 `0xFA` / 写入 `0xFB`，索引 3 为命令号，索引 4 为设定值，其余字节为零。命令 13 返回 CPU/GPU 两个小端 16 位 RPM；20 控制开关，21 设置以 100 RPM 为单位的值。Linux 模块将同一请求传至 GUID 对应的 WMI 方法，实例 0、方法 1。

原程序的设置读取边界规则保留在 `src/protocol.rs`；`tests/fixtures/original-request-vectors.json` 保存从原 DLL 请求构造函数取得的 512 组向量，供 Rust 差分测试。

常规 `cargo test` 包含 9 项自动测试，覆盖原协议、模式边界、固定滑块刻度与模式范围限制、原设置兼容性、应用偏好保存与默认值、主题跟随规则，原 DLL 的 512 组差分向量，以及可变字体作为主字体和回退字体时的字重栅格化。实机测试使用 `/dev/jiaolong-mifs`，写入风扇并短暂切换当前电源模式，结束时恢复原状态；需要在游戏模式且自定义模式关闭时运行：

```bash
cargo test --locked hardware_controller_roundtrip -- --ignored --nocapture
```

Windows 原 WMI 后端也保留在 `src/transport.rs`，Linux MVP 使用 `/dev/jiaolong-mifs`。
