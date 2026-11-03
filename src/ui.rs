use super::{Command, Snapshot, View};
use gpui::{
    AssetSource, Context, Div, FontWeight, IntoElement, MouseButton, ParentElement, Render,
    SharedString, Stateful, Styled, Window, WindowAppearance, canvas, deferred, div, prelude::*,
    px, relative, rgb, svg,
};
use jiaolong_control::{
    preferences::{Language, Preferences, Theme},
    protocol::{Mode, fan_fraction},
};
use std::borrow::Cow;

// GPUI's Linux .SystemUIFont maps to a fixed family. Resolve the
// fontconfig sans-serif alias so the user's configured default is honored.
pub fn system_font() -> SharedString {
    #[cfg(target_os = "linux")]
    if let Ok(output) = std::process::Command::new("fc-match")
        .args(["-f", "%{family}", "sans-serif"])
        .output()
        && output.status.success()
        && let Ok(family) = String::from_utf8(output.stdout)
        && let Some(family) = family.split(',').next()
        && !family.trim().is_empty()
    {
        return family.trim().to_owned().into();
    }
    ".SystemUIFont".into()
}

pub struct Assets;
impl AssetSource for Assets {
    fn load(&self, path: &str) -> anyhow::Result<Option<Cow<'static, [u8]>>> {
        let paths = match path {
            "fan" => {
                r#"<circle cx="12" cy="12" r="2"/><path d="M12 10C8 7 8 2 12 2c5 0 6 5 2 9M14 12c5-1 9 2 7 6-3 4-8 2-9-4M10 13c-1 5-6 7-8 3-2-5 3-7 8-4"/>"#
            }
            "overview" => {
                r#"<rect x="3" y="3" width="7" height="7" rx="1"/><rect x="14" y="3" width="7" height="7" rx="1"/><rect x="3" y="14" width="7" height="7" rx="1"/><rect x="14" y="14" width="7" height="7" rx="1"/>"#
            }
            "performance" => {
                r#"<path d="M3 6h6m4 0h8M3 12h12m4 0h2M3 18h2m4 0h12M9 3v6m6 0v6M5 15v6"/>"#
            }
            "keyboard" => {
                r#"<rect x="2" y="5" width="20" height="14" rx="2"/><path d="M6 9h1m3 0h1m3 0h1m3 0h1M6 12h1m3 0h1m3 0h1m3 0h1M7 16h10"/>"#
            }
            "devices" => {
                r#"<path d="M3 6h4m4 0h10M3 18h10m4 0h4"/><circle cx="9" cy="6" r="2"/><circle cx="15" cy="18" r="2"/>"#
            }
            "settings" => {
                r#"<path d="m9 3-1 3-3 1 1 3-2 2 2 2-1 3 3 1 1 3h6l1-3 3-1-1-3 2-2-2-2 1-3-3-1-1-3z"/><circle cx="12" cy="12" r="3"/>"#
            }
            "cpu" => {
                r#"<rect x="5" y="5" width="14" height="14" rx="2"/><rect x="9" y="9" width="6" height="6"/><path d="M9 2v3m6-3v3M9 19v3m6-3v3M2 9h3m-3 6h3m14-6h3m-3 6h3"/>"#
            }
            "gpu" => {
                r#"<rect x="3" y="5" width="18" height="13" rx="1"/><circle cx="10" cy="11" r="3"/><path d="M16 9h2m-2 4h2M5 18v3m4-3v3m4-3v3"/>"#
            }
            "refresh" => {
                r#"<path d="M20 7v5h-5M4 17v-5h5M5 7a8 8 0 0 1 13-2l2 3M4 16l2 3a8 8 0 0 0 13-2"/>"#
            }
            "clock" => r#"<circle cx="12" cy="12" r="9"/><path d="M12 6v6l4 2"/>"#,
            "monitor" => {
                r#"<rect x="3" y="3" width="18" height="14" rx="2"/><path d="M12 17v4m-4 0h8"/>"#
            }
            "sun" => {
                r#"<circle cx="12" cy="12" r="4"/><path d="M12 2v2m0 16v2M2 12h2m16 0h2M5 5l1 1m12 12 1 1M5 19l1-1M18 6l1-1"/>"#
            }
            "moon" => r#"<path d="M20 14A9 9 0 0 1 10 3a9 9 0 1 0 10 11Z"/>"#,
            "check" => r#"<circle cx="12" cy="12" r="9"/><path d="m8 12 3 3 5-6"/>"#,
            "circle" => r#"<circle cx="12" cy="12" r="9"/>"#,
            "down" => r#"<path d="m6 9 6 6 6-6"/>"#,
            _ => return Ok(None),
        };
        Ok(Some(Cow::Owned(format!(r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="white" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round">{paths}</svg>"#).into_bytes())))
    }
    fn list(&self, _: &str) -> anyhow::Result<Vec<SharedString>> {
        Ok(Vec::new())
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Overview,
    Fan,
    Performance,
    Keyboard,
    Devices,
    Settings,
}
impl Page {
    fn label(self, lang: Language) -> &'static str {
        match self {
            Self::Overview => lang.text("设备概览", "Overview"),
            Self::Fan => lang.text("风扇控制", "Fan control"),
            Self::Performance => lang.text("性能调节", "Performance"),
            Self::Keyboard => lang.text("键盘与灯光", "Keyboard & lighting"),
            Self::Devices => lang.text("设备设置", "Device settings"),
            Self::Settings => lang.text("应用设置", "App settings"),
        }
    }
    fn icon(self) -> &'static str {
        match self {
            Self::Overview => "overview",
            Self::Fan => "fan",
            Self::Performance => "performance",
            Self::Keyboard => "keyboard",
            Self::Devices => "devices",
            Self::Settings => "settings",
        }
    }
}

#[derive(Clone, Copy)]
struct Palette {
    bg: u32,
    card: u32,
    sidebar: u32,
    border: u32,
    fg: u32,
    secondary: u32,
    muted: u32,
    blue: u32,
    selected: u32,
    raised: u32,
    green: u32,
    green_bg: u32,
    red: u32,
    red_bg: u32,
}
impl Palette {
    fn new(dark: bool) -> Self {
        if dark {
            Self {
                bg: 0x242424,
                card: 0x303030,
                sidebar: 0x303030,
                border: 0x454545,
                fg: 0xffffff,
                secondary: 0xdedede,
                muted: 0xb0b0b0,
                blue: 0x78aeed,
                selected: 0x253d59,
                raised: 0x454545,
                green: 0x8ff0a4,
                green_bg: 0x253d30,
                red: 0xf66151,
                red_bg: 0x492b2d,
            }
        } else {
            Self {
                bg: 0xfafafa,
                card: 0xffffff,
                sidebar: 0xebebeb,
                border: 0xd9d9d9,
                fg: 0x2e3436,
                secondary: 0x50565a,
                muted: 0x62686c,
                blue: 0x3584e4,
                selected: 0xe1edfa,
                raised: 0xebebeb,
                green: 0x1b8554,
                green_bg: 0xe2f3e9,
                red: 0xc01c28,
                red_bg: 0xfbe5e7,
            }
        }
    }
}
fn icon(name: &'static str, color: u32, pixels: f32) -> impl IntoElement {
    svg()
        .path(name)
        .size(px(pixels))
        .flex_shrink_0()
        .text_color(rgb(color))
}
fn text(content: impl Into<SharedString>, size: f32, color: u32) -> Div {
    div()
        .text_size(px(size))
        .text_color(rgb(color))
        .child(content.into())
}
fn card(p: Palette) -> Div {
    div()
        .bg(rgb(p.card))
        .border_1()
        .border_color(rgb(p.border))
        .rounded(px(12.))
}
fn button(id: &'static str, p: Palette) -> Stateful<Div> {
    div()
        .id(id)
        .tab_index(0)
        .h(px(40.))
        .px(px(12.))
        .rounded(px(8.))
        .bg(rgb(p.raised))
        .flex()
        .items_center()
        .justify_center()
        .gap(px(8.))
        .cursor_pointer()
        .hover(move |style| style.bg(rgb(p.selected)))
}
fn progress(value: f32, p: Palette) -> Div {
    div()
        .w_full()
        .h(px(4.))
        .rounded(px(2.))
        .bg(rgb(p.selected))
        .overflow_hidden()
        .child(
            div()
                .h_full()
                .w(relative((value / 100.).clamp(0., 1.)))
                .bg(rgb(p.blue))
                .rounded(px(2.)),
        )
}

impl View {
    fn drag_fan_target(&mut self, position: gpui::Point<gpui::Pixels>, cx: &mut Context<Self>) {
        let Some((mode, _)) = self.slider_drag else {
            return;
        };
        if self.pending || self.state.as_ref().is_none_or(|s| s.fan_mode() != mode) {
            self.slider_drag = None;
        } else {
            let bounds = self.slider_bounds.get();
            let fraction =
                f32::from(position.x - bounds.origin.x) / f32::from(bounds.size.width).max(1.);
            self.slider_drag = Some((mode, mode.speed_at_fraction(fraction)));
        }
        cx.notify();
    }
    fn commit_fan_target(&mut self, cx: &mut Context<Self>) {
        if let Some((mode, speed)) = self.slider_drag.take() {
            if !self.pending
                && self
                    .state
                    .as_ref()
                    .is_some_and(|s| s.fan_mode() == mode && s.speed != speed)
            {
                self.send(Command::SetSpeed(speed), cx);
            }
            cx.notify();
        }
    }
    fn fan_slider(&self, p: Palette, cx: &mut Context<Self>) -> Stateful<Div> {
        let state = self.state.as_ref();
        let enabled = state.is_some() && !self.pending;
        let (min, max) = state.map(|s| s.fan_mode().limits()).unwrap_or((22, 58));
        let speed = self
            .slider_drag
            .map(|(_, speed)| speed)
            .or_else(|| state.map(|s| s.speed))
            .unwrap_or(22);
        let bounds = self.slider_bounds.clone();
        div()
            .id("fan-slider")
            .track_focus(&self.slider_focus)
            .tab_index(0)
            .w_full()
            .h(px(28.))
            .px(px(10.))
            .rounded(px(6.))
            .focus_visible(move |style| style.bg(rgb(p.selected)))
            .when(enabled, |d| d.cursor_pointer())
            .when(!enabled, |d| d.opacity(0.35))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |v, event: &gpui::MouseDownEvent, window, cx| {
                    if enabled
                        && !v.pending
                        && let Some(state) = &v.state
                    {
                        v.slider_drag = Some((state.fan_mode(), state.speed));
                        window.focus(&v.slider_focus, cx);
                        v.drag_fan_target(event.position, cx);
                        cx.stop_propagation();
                    }
                }),
            )
            .on_key_down(cx.listener(|v, event: &gpui::KeyDownEvent, _, cx| {
                if v.pending {
                    return;
                }
                let Some(state) = &v.state else {
                    return;
                };
                let (min, max) = state.fan_mode().limits();
                let speed = match event.keystroke.key.as_str() {
                    "left" | "down" => state.speed.saturating_sub(1).max(min),
                    "right" | "up" => state.speed.saturating_add(1).min(max),
                    "home" => min,
                    "end" => max,
                    _ => return,
                };
                if speed != state.speed {
                    v.send(Command::SetSpeed(speed), cx);
                }
                cx.stop_propagation();
            }))
            .child(
                div()
                    .relative()
                    .size_full()
                    .child(
                        canvas(move |b, _, _| bounds.set(b), |_, _, _, _| {})
                            .absolute()
                            .size_full(),
                    )
                    .child(
                        div()
                            .absolute()
                            .top(px(11.))
                            .w_full()
                            .h(px(6.))
                            .rounded(px(3.))
                            .bg(rgb(p.raised)),
                    )
                    .when(state.is_some(), |d| {
                        d.child(
                            div()
                                .absolute()
                                .top(px(11.))
                                .left(relative(fan_fraction(min)))
                                .w(relative(fan_fraction(max) - fan_fraction(min)))
                                .h(px(6.))
                                .rounded(px(3.))
                                .bg(rgb(p.selected)),
                        )
                        .child(
                            div()
                                .absolute()
                                .top(px(11.))
                                .left(relative(fan_fraction(min)))
                                .w(relative((fan_fraction(speed) - fan_fraction(min)).max(0.)))
                                .h(px(6.))
                                .rounded(px(3.))
                                .bg(rgb(p.blue)),
                        )
                        .child(
                            div()
                                .absolute()
                                .top(px(4.))
                                .left(relative(fan_fraction(speed)))
                                .ml(px(-10.))
                                .size(px(20.))
                                .rounded_full()
                                .bg(rgb(p.card))
                                .border(px(3.))
                                .border_color(rgb(p.blue)),
                        )
                    }),
            )
    }
    fn t(&self, chinese: &'static str, english: &'static str) -> &'static str {
        self.preferences.language.text(chinese, english)
    }
    fn update_preferences(&mut self, preferences: Preferences, cx: &mut Context<Self>) {
        match if self.preview {
            Ok(())
        } else {
            preferences.save(&self.preferences_path)
        } {
            Ok(()) => {
                self.preferences = preferences;
                self.preference_error = None;
            }
            Err(error) => self.preference_error = Some(format!("{error:#}")),
        }
        self.language_menu_open = false;
        cx.notify();
    }
    fn navigation(&self, p: Palette, cx: &mut Context<Self>) -> Div {
        let mut nav = div()
            .w(px(match self.preferences.language {
                Language::ZhCn => 208.,
                Language::En => 280.,
            }))
            .h_full()
            .flex_shrink_0()
            .bg(rgb(p.sidebar))
            .border_r_1()
            .border_color(rgb(p.border))
            .p(px(16.))
            .pt(px(24.))
            .pb(px(24.))
            .flex()
            .flex_col()
            .gap(px(10.))
            .child(text(self.t("工作空间", "Workspace"), 11., p.muted).h(px(24.)));
        for (i, page) in [
            Page::Overview,
            Page::Fan,
            Page::Performance,
            Page::Keyboard,
            Page::Devices,
            Page::Settings,
        ]
        .into_iter()
        .enumerate()
        {
            if page == Page::Settings {
                nav = nav.child(div().flex_1());
            }
            let active = self.page == page;
            let color = if active { p.blue } else { p.secondary };
            let mut row = div()
                .id(("nav", i))
                .tab_index(0)
                .w_full()
                .h(px(44.))
                .flex_shrink_0()
                .px(px(12.))
                .flex()
                .items_center()
                .gap(px(10.))
                .rounded(px(8.))
                .bg(rgb(if active { p.selected } else { p.sidebar }))
                .cursor_pointer()
                .hover(move |style| style.bg(rgb(if active { p.selected } else { p.raised })))
                .child(icon(page.icon(), color, 16.))
                .child(
                    text(page.label(self.preferences.language), 13., color)
                        .flex_1()
                        .min_w(px(0.))
                        .whitespace_nowrap()
                        .when(active, |d| d.font_weight(FontWeight::SEMIBOLD)),
                );
            if !matches!(page, Page::Fan | Page::Settings) {
                row = row.child(text(self.t("预留", "Planned"), 10., p.muted).flex_shrink_0());
            }
            nav = nav.child(row.on_click(cx.listener(move |v, _, _, cx| {
                v.slider_drag = None;
                v.page = page;
                v.language_menu_open = false;
                cx.notify();
            })));
        }
        nav
    }
    fn metric(&self, cpu: bool, state: Option<&Snapshot>, p: Palette) -> Div {
        let rpm = state.map(|s| if cpu { s.cpu } else { s.gpu });
        let mut head = div()
            .flex()
            .items_center()
            .gap(px(8.))
            .child(icon(if cpu { "cpu" } else { "gpu" }, p.muted, 16.))
            .child(
                text(
                    if cpu {
                        self.t("CPU 风扇", "CPU fan")
                    } else {
                        self.t("GPU 风扇", "GPU fan")
                    },
                    12.,
                    p.secondary,
                )
                .flex_1(),
            );
        if cpu {
            head = head.child(
                text(
                    rpm.map(|rpm| format!("{:.0}%", rpm as f32 / 60.))
                        .unwrap_or_else(|| "—".into()),
                    13.,
                    p.blue,
                )
                .font_weight(FontWeight::SEMIBOLD),
            );
        }
        card(p)
            .flex_1()
            .h(px(120.))
            .p(px(16.))
            .flex()
            .flex_col()
            .justify_between()
            .child(head)
            .child(
                div()
                    .flex()
                    .items_end()
                    .gap(px(16.))
                    .child(text(
                        rpm.map(|n| n.to_string()).unwrap_or_else(|| "—".into()),
                        34.,
                        p.fg,
                    ))
                    .child(text("RPM", 11., p.muted).pb(px(5.))),
            )
            .child(if cpu {
                progress(rpm.map(|n| n as f32 / 60.).unwrap_or(0.), p)
            } else {
                text(self.t("实际转速", "Actual speed"), 11., p.muted)
            })
    }
    fn mode_label(&self, state: &Snapshot) -> &'static str {
        if state.custom {
            return self.t("自定义模式", "Custom mode");
        }
        match state.mode {
            Mode::Performance => self.t("狂暴模式", "Performance mode"),
            Mode::Balanced => self.t("游戏模式", "Game mode"),
            Mode::Quiet => self.t("办公模式", "Office mode"),
        }
    }
    fn fan_page(&self, p: Palette, cx: &mut Context<Self>) -> Div {
        let state = self.state.as_ref();
        let connected = state.is_some();
        let enabled = connected && !self.pending;
        let mut modes = div().flex_1().flex().gap(px(8.));
        for (id, mode, label) in [
            ("mode-office", Mode::Quiet, self.t("办公", "Office")),
            ("mode-game", Mode::Balanced, self.t("游戏", "Game")),
            (
                "mode-performance",
                Mode::Performance,
                self.t("狂暴", "Performance"),
            ),
        ] {
            let selected = state.is_some_and(|s| !s.custom && s.mode == mode);
            modes = modes.child(
                button(id, p)
                    .flex_1()
                    .h(px(36.))
                    .bg(rgb(if selected { p.selected } else { p.raised }))
                    .text_color(rgb(if selected { p.blue } else { p.secondary }))
                    .when(selected, |d| d.font_weight(FontWeight::SEMIBOLD))
                    .when(!enabled, |d| d.opacity(0.35).cursor_default())
                    .child(label)
                    .on_click(cx.listener(move |v, _, _, cx| {
                        if enabled && !selected && !v.pending && v.state.is_some() {
                            v.send(Command::SetMode(mode), cx);
                        }
                    })),
            );
        }
        let mode_picker = card(p)
            .flex_shrink_0()
            .p(px(12.))
            .flex()
            .items_center()
            .gap(px(12.))
            .child(text(self.t("运行模式", "Operating mode"), 14., p.secondary).flex_shrink_0())
            .child(modes);
        let header = div()
            .h(px(56.))
            .flex_shrink_0()
            .flex()
            .justify_between()
            .items_center()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(
                        text(self.t("风扇控制", "Fan control"), 26., p.fg)
                            .font_weight(FontWeight::BOLD),
                    )
                    .child(text(
                        self.t(
                            "查看实时转速，调整风扇设定",
                            "Monitor fan speed and adjust the target",
                        ),
                        13.,
                        p.secondary,
                    )),
            )
            .child(
                div()
                    .px(px(12.))
                    .py(px(7.))
                    .rounded(px(20.))
                    .bg(rgb(if connected {
                        p.green_bg
                    } else if self.loading {
                        p.raised
                    } else {
                        p.red_bg
                    }))
                    .child(text(
                        if self.pending || self.loading {
                            self.t("正在读取…", "Reading…")
                        } else if connected {
                            self.t("●  设备已连接", "●  Connected")
                        } else {
                            self.t("●  连接待恢复", "●  Reconnecting")
                        },
                        11.,
                        if connected {
                            p.green
                        } else if self.loading {
                            p.muted
                        } else {
                            p.red
                        },
                    )),
            );
        let (min, max) = state.map(|s| s.fan_mode().limits()).unwrap_or((0, 0));
        let step = |id: &'static str, label: &'static str, delta: i8, allowed: bool| {
            button(id, p)
                .w(px(36.))
                .h(px(36.))
                .text_size(px(20.))
                .bg(rgb(if delta > 0 { 0x3584e4 } else { p.raised }))
                .text_color(rgb(if delta > 0 { 0xffffff } else { p.fg }))
                .when(!allowed, |d| d.opacity(0.35).cursor_default())
                .child(label)
                .on_click(cx.listener(move |v, _, _, cx| {
                    if allowed && v.state.is_some() && !v.pending {
                        v.send(Command::Adjust(delta), cx);
                    }
                }))
        };
        let setting = card(p)
            .h(px(176.))
            .flex_shrink_0()
            .p(px(20.))
            .flex()
            .flex_col()
            .justify_between()
            .child(
                div()
                    .flex()
                    .justify_between()
                    .items_center()
                    .child(text(self.t("风扇设定", "Fan target"), 14., p.secondary))
                    .child(
                        div()
                            .px(px(12.))
                            .py(px(5.))
                            .rounded(px(6.))
                            .bg(rgb(if connected { p.selected } else { p.raised }))
                            .child(text(
                                state
                                    .map(|s| {
                                        format!(
                                            "{} · {}–{} RPM",
                                            self.mode_label(s),
                                            min as u16 * 100,
                                            max as u16 * 100
                                        )
                                    })
                                    .unwrap_or_else(|| {
                                        self.t("模式待读取", "Awaiting mode").into()
                                    }),
                                11.,
                                if connected { p.blue } else { p.muted },
                            )),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .items_end()
                            .gap(px(16.))
                            .child(text(
                                state
                                    .map(|s| {
                                        (self.slider_drag.map(|(_, speed)| speed).unwrap_or(s.speed)
                                            as u16
                                            * 100)
                                            .to_string()
                                    })
                                    .unwrap_or_else(|| "—".into()),
                                46.,
                                p.fg,
                            ))
                            .child(text("RPM", 12., p.muted).pb(px(8.))),
                    )
                    .child(
                        div()
                            .flex()
                            .gap(px(12.))
                            .child(step(
                                "sub",
                                "−",
                                -1,
                                enabled && state.is_some_and(|s| s.speed > min),
                            ))
                            .child(step(
                                "add",
                                "+",
                                1,
                                enabled && state.is_some_and(|s| s.speed < max),
                            )),
                    ),
            )
            .child(self.fan_slider(p, cx))
            .child(
                div()
                    .relative()
                    .h(px(14.))
                    .mx(px(10.))
                    .text_size(px(10.))
                    .text_color(rgb(p.muted))
                    .child(div().absolute().left_0().child("2200"))
                    .child(
                        div()
                            .absolute()
                            .left(relative(fan_fraction(35)))
                            .ml(px(-14.))
                            .child("3500"),
                    )
                    .child(
                        div()
                            .absolute()
                            .left(relative(fan_fraction(50)))
                            .ml(px(-14.))
                            .child("5000"),
                    )
                    .child(div().absolute().right_0().child("5800 RPM")),
            );
        let on = state.is_some_and(|s| s.boost);
        let switch = div()
            .id("toggle")
            .tab_index(0)
            .w(px(42.))
            .h(px(24.))
            .p(px(2.))
            .rounded(px(12.))
            .flex()
            .items_center()
            .when(on, |d| d.justify_end())
            .bg(rgb(if on { 0x3584e4 } else { 0x77767b }))
            .cursor_pointer()
            .when(!enabled, |d| d.opacity(0.35).cursor_default())
            .child(div().size(px(20.)).rounded(px(10.)).bg(rgb(0xffffff)))
            .on_click(cx.listener(|v, _, _, cx| {
                if !v.pending
                    && let Some(s) = &v.state
                {
                    v.send(Command::Toggle(!s.boost), cx);
                }
            }));
        let maximum = card(p)
            .h(px(80.))
            .flex_shrink_0()
            .p(px(20.))
            .flex()
            .items_center()
            .justify_between()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(6.))
                    .child(
                        text(self.t("最大风扇", "Maximum fan"), 14., p.fg)
                            .font_weight(FontWeight::SEMIBOLD),
                    )
                    .child(text(
                        if connected {
                            self.t(
                                "调节转速时自动开启",
                                "Enabled automatically when adjusting speed",
                            )
                        } else {
                            self.t(
                                "请检查固件访问接口，点击刷新重试",
                                "Check firmware access and refresh to retry",
                            )
                        },
                        12.,
                        p.muted,
                    )),
            )
            .child(switch);
        let mut body = div()
            .flex_1()
            .min_h(px(0.))
            .id("fan-body")
            .overflow_y_scroll()
            .p(px(24.))
            .flex()
            .flex_col()
            .gap(px(16.))
            .child(header)
            .child(mode_picker)
            .child(
                div()
                    .flex()
                    .gap(px(16.))
                    .flex_shrink_0()
                    .child(self.metric(true, state, p))
                    .child(self.metric(false, state, p)),
            )
            .child(setting)
            .child(maximum);
        if !self.loading && !self.pending && !self.preview && self.status != "已连接" {
            body = body.child(text(self.status.clone(), 11., p.red));
        }
        div().size_full().flex().flex_col().child(body).child(
            div()
                .h(px(44.))
                .flex_shrink_0()
                .px(px(24.))
                .border_t_1()
                .border_color(rgb(p.border))
                .flex()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .flex()
                        .gap(px(7.))
                        .items_center()
                        .child(icon("clock", p.muted, 12.))
                        .child(text(
                            if self.preview {
                                self.t("设计预览 · 示例数据", "Design preview · Sample data")
                            } else {
                                self.t("每 10 秒自动刷新", "Refreshes every 10 seconds")
                            },
                            11.,
                            p.muted,
                        )),
                )
                .child(
                    button("refresh", p)
                        .h(px(28.))
                        .when(self.pending, |d| d.opacity(0.5).cursor_default())
                        .child(icon("refresh", p.secondary, 12.))
                        .child(text(self.t("刷新", "Refresh"), 11., p.secondary))
                        .on_click(cx.listener(|v, _, _, cx| {
                            if !v.pending {
                                v.send(Command::Refresh, cx);
                            }
                        })),
                ),
        )
    }
    fn settings_page(&self, p: Palette, dark: bool, cx: &mut Context<Self>) -> Stateful<Div> {
        let mut options = div().flex().gap(px(12.)).h(px(128.)).flex_shrink_0();
        for (index, theme, glyph, label) in [
            (0, Theme::System, "monitor", self.t("跟随系统", "System")),
            (1, Theme::Light, "sun", self.t("浅色", "Light")),
            (2, Theme::Dark, "moon", self.t("深色", "Dark")),
        ] {
            let selected = self.preferences.theme == theme;
            options = options.child(
                div()
                    .id(("theme", index as usize))
                    .tab_index(0)
                    .flex_1()
                    .h_full()
                    .p(px(16.))
                    .rounded(px(12.))
                    .border(px(if selected { 2. } else { 1. }))
                    .border_color(rgb(if selected { p.blue } else { p.border }))
                    .bg(rgb(if selected { p.selected } else { p.card }))
                    .flex()
                    .flex_col()
                    .gap(px(20.))
                    .cursor_pointer()
                    .hover(move |style| style.bg(rgb(p.selected)))
                    .child(
                        div()
                            .flex()
                            .justify_between()
                            .items_center()
                            .child(icon(glyph, p.blue, 24.))
                            .child(icon(
                                if selected { "check" } else { "circle" },
                                if selected { p.blue } else { p.muted },
                                20.,
                            )),
                    )
                    .child(
                        text(label, 14., p.fg)
                            .when(selected, |d| d.font_weight(FontWeight::SEMIBOLD)),
                    )
                    .on_click(cx.listener(move |v, _, _, cx| {
                        let mut preferences = v.preferences.clone();
                        preferences.theme = theme;
                        v.update_preferences(preferences, cx);
                    })),
            );
        }
        let mut language = div().relative().w(px(192.)).flex_shrink_0().child(
            button("language", p)
                .w_full()
                .justify_between()
                .child(text(
                    match self.preferences.language {
                        Language::ZhCn => "简体中文",
                        Language::En => "English",
                    },
                    13.,
                    p.fg,
                ))
                .child(icon("down", p.secondary, 16.))
                .on_click(cx.listener(|v, _, _, cx| {
                    v.language_menu_open = !v.language_menu_open;
                    cx.notify();
                })),
        );
        if self.language_menu_open {
            let mut menu = card(p)
                .absolute()
                .top(px(44.))
                .left_0()
                .w_full()
                .p(px(4.))
                .shadow_md()
                .flex()
                .flex_col();
            for (index, lang, label) in [
                (0, Language::ZhCn, "简体中文"),
                (1, Language::En, "English"),
            ] {
                menu = menu.child(
                    div()
                        .id(("language-option", index as usize))
                        .tab_index(0)
                        .h(px(36.))
                        .px(px(8.))
                        .rounded(px(6.))
                        .flex()
                        .items_center()
                        .justify_between()
                        .cursor_pointer()
                        .hover(move |style| style.bg(rgb(p.selected)))
                        .child(text(label, 13., p.fg))
                        .when(self.preferences.language == lang, |d| {
                            d.child(icon("check", p.blue, 16.))
                        })
                        .on_click(cx.listener(move |v, _, _, cx| {
                            let mut preferences = v.preferences.clone();
                            preferences.language = lang;
                            v.update_preferences(preferences, cx);
                        })),
                );
            }
            language = language.child(
                deferred(menu.on_mouse_down_out(cx.listener(|v, _, _, cx| {
                    v.language_menu_open = false;
                    cx.notify();
                })))
                .with_priority(1),
            );
        }
        let theme_note = if self.preferences.theme == Theme::System {
            if dark {
                self.t(
                    "跟随桌面设置 · 当前为深色外观",
                    "Follows desktop settings · Currently dark",
                )
            } else {
                self.t(
                    "跟随桌面设置 · 当前为浅色外观",
                    "Follows desktop settings · Currently light",
                )
            }
        } else {
            self.t("使用所选外观", "Uses the selected appearance")
        };
        let mut body = div()
            .id("settings-body")
            .size_full()
            .overflow_y_scroll()
            .p(px(24.))
            .flex()
            .flex_col()
            .gap(px(16.))
            .child(
                text(self.t("应用设置", "App settings"), 26., p.fg).font_weight(FontWeight::BOLD),
            )
            .child(text(
                self.t(
                    "调整控制中心的外观与界面语言",
                    "Customize appearance and interface language",
                ),
                13.,
                p.secondary,
            ))
            .child(
                text(self.t("外观", "Appearance"), 16., p.fg)
                    .font_weight(FontWeight::SEMIBOLD)
                    .mt(px(4.)),
            )
            .child(options)
            .child(text(theme_note, 12., p.muted))
            .child(
                text(self.t("语言", "Language"), 16., p.fg)
                    .font_weight(FontWeight::SEMIBOLD)
                    .mt(px(4.)),
            )
            .child(
                card(p)
                    .h(px(88.))
                    .flex_shrink_0()
                    .p(px(20.))
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(20.))
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .gap(px(6.))
                            .child(
                                text(self.t("界面语言", "Interface language"), 14., p.fg)
                                    .font_weight(FontWeight::MEDIUM),
                            )
                            .child(text(
                                self.t(
                                    "选择控制中心使用的语言",
                                    "Choose the language for Control Center",
                                ),
                                12.,
                                p.muted,
                            )),
                    )
                    .child(language),
            )
            .child(text(
                self.t(
                    "可选语言：简体中文 / English",
                    "Available languages: 简体中文 / English",
                ),
                12.,
                p.muted,
            ))
            .child(text(
                self.t(
                    "更改即时生效，并在下次启动时保留",
                    "Changes apply immediately and are saved for next launch",
                ),
                12.,
                p.muted,
            ));
        if let Some(error) = &self.preference_error {
            body = body.child(text(
                format!(
                    "{}: {error}",
                    self.t("应用偏好读写失败", "Application preferences error")
                ),
                12.,
                p.red,
            ));
        }
        body
    }
    fn reserved_page(&self, p: Palette) -> Stateful<Div> {
        let lang = self.preferences.language;
        let mut body = div()
            .id("reserved-body")
            .size_full()
            .overflow_y_scroll()
            .p(px(24.))
            .flex()
            .flex_col()
            .gap(px(16.))
            .child(
                text(self.t("功能扩展空间", "Feature expansion"), 26., p.fg)
                    .font_weight(FontWeight::BOLD),
            )
            .child(text(
                self.t(
                    "按原程序功能组织 · 各模块预留独立页面与控件区域",
                    "Original features · Reserved pages and controls",
                ),
                13.,
                p.secondary,
            ));
        for pages in [
            [Page::Overview, Page::Performance],
            [Page::Keyboard, Page::Devices],
        ] {
            let mut row = div().flex().gap(px(16.)).flex_shrink_0();
            for page in pages {
                let description = match page {
                    Page::Overview => self.t("CPU / GPU 温度、频率与占用\n处理器名称与核心数\n内存占用与频率 · 存储用量", "CPU / GPU temperature, frequency and usage\nProcessor name and core count\nMemory usage / frequency · Storage usage"),
                    Page::Performance => self.t("办公 / 游戏 / 狂暴 / 自定义模式\nCPU 温度墙 · SPL / SPPT 功耗\nGPU 核心频率偏移", "Office / Game / Performance / Custom modes\nCPU thermal limit · SPL / SPPT power\nGPU core clock offset"),
                    Page::Keyboard => self.t("背光亮度 · RGB 颜色与灯效\n氛围灯控制\nFn / Num / Caps 锁定状态", "Backlight brightness · RGB colors and effects\nAmbient lighting\nFn / Num / Caps lock state"),
                    _ => self.t("独显直连\n触摸板开关\nOSD 图标开关", "Discrete graphics direct mode\nTouchpad toggle\nOSD icon toggle"),
                };
                row = row.child(
                    card(p)
                        .flex_1()
                        .h(px(232.))
                        .p(px(20.))
                        .flex()
                        .flex_col()
                        .gap(px(14.))
                        .when(self.page == page, |d| d.border_color(rgb(p.blue)))
                        .child(icon(page.icon(), p.blue, 22.))
                        .child(
                            text(
                                format!("{} / {}", page.label(lang), self.t("预留", "Planned")),
                                17.,
                                p.fg,
                            )
                            .font_weight(FontWeight::SEMIBOLD),
                        )
                        .child(text(description, 13., p.secondary).flex_1())
                        .child(text(
                            self.t(
                                "独立页面 · 数据与操作区域",
                                "Dedicated page · Data and controls",
                            ),
                            11.,
                            p.muted,
                        )),
                );
            }
            body = body.child(row);
        }
        body
    }
}

impl Render for View {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if window.focused(cx).is_none() {
            window.focus(&self.focus, cx);
        }
        let system_dark = matches!(
            window.appearance(),
            WindowAppearance::Dark | WindowAppearance::VibrantDark
        );
        let dark = self.preferences.theme.is_dark(system_dark);
        let p = Palette::new(dark);
        let content = match self.page {
            Page::Fan => self.fan_page(p, cx),
            Page::Settings => div().size_full().child(self.settings_page(p, dark, cx)),
            _ => div().size_full().child(self.reserved_page(p)),
        };
        div()
            .id("app-root")
            .track_focus(&self.focus)
            .on_mouse_move(cx.listener(|v, event: &gpui::MouseMoveEvent, _, cx| {
                if v.slider_drag.is_some() {
                    if event.pressed_button == Some(MouseButton::Left) {
                        v.drag_fan_target(event.position, cx);
                    } else {
                        v.slider_drag = None;
                        cx.notify();
                    }
                }
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|v, event: &gpui::MouseUpEvent, _, cx| {
                    if v.slider_drag.is_some() {
                        v.drag_fan_target(event.position, cx);
                    }
                    v.commit_fan_target(cx);
                }),
            )
            .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|v, event: &gpui::MouseUpEvent, _, cx| {
                    if v.slider_drag.is_some() {
                        v.drag_fan_target(event.position, cx);
                    }
                    v.commit_fan_target(cx);
                }),
            )
            .on_key_down(cx.listener(|v, event: &gpui::KeyDownEvent, window, cx| {
                if event.keystroke.key == "tab" {
                    v.slider_drag = None;
                    if event.keystroke.modifiers.shift {
                        window.focus_prev(cx);
                    } else {
                        window.focus_next(cx);
                    }
                    cx.stop_propagation();
                }
                if event.keystroke.key == "escape" {
                    v.slider_drag = None;
                    v.language_menu_open = false;
                    cx.notify();
                }
            }))
            .size_full()
            .bg(rgb(p.bg))
            .text_color(rgb(p.fg))
            .font_family(self.system_font.clone())
            .text_size(px(13.))
            .flex()
            .flex_col()
            .child(
                div()
                    .h(px(32.))
                    .flex_shrink_0()
                    .px(px(20.))
                    .border_b_1()
                    .border_color(rgb(p.border))
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .child(icon("fan", p.blue, 18.))
                    .child(
                        text("Jiaolong Control Center", 14., p.secondary)
                            .font_weight(FontWeight::SEMIBOLD),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_h(px(0.))
                    .flex()
                    .child(self.navigation(p, cx))
                    .child(div().flex_1().min_w(px(0.)).h_full().child(content)),
            )
    }
}
