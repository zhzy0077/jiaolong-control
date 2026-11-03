mod ui;

use anyhow::{Result, ensure};
use gpui::{App, AppContext, Bounds, Context, Window, WindowBounds, WindowOptions, px, size};
use jiaolong_control::{
    preferences::{Preferences, preferences_path},
    protocol::{self, Mode},
    settings::Settings,
    transport::Transport,
};
use std::{
    sync::mpsc::{self, Receiver, Sender},
    time::Duration,
};

enum Command {
    Refresh,
    SetMode(Mode),
    Toggle(bool),
    Adjust(i8),
    SetSpeed(u8),
}
#[derive(Clone)]
struct Snapshot {
    mode: Mode,
    custom: bool,
    speed: u8,
    boost: bool,
    cpu: u16,
    gpu: u16,
}

impl Snapshot {
    fn fan_mode(&self) -> Mode {
        if self.custom {
            Mode::Performance
        } else {
            self.mode
        }
    }
}
struct View {
    tx: Sender<Command>,
    rx: Receiver<Result<Snapshot, String>>,
    state: Option<Snapshot>,
    status: String,
    system_font: gpui::SharedString,
    preview: bool,
    focus: gpui::FocusHandle,
    loading: bool,
    pending: bool,
    preferences: Preferences,
    preferences_path: std::path::PathBuf,
    preference_error: Option<String>,
    page: ui::Page,
    language_menu_open: bool,
    slider_focus: gpui::FocusHandle,
    slider_bounds: std::rc::Rc<std::cell::Cell<Bounds<gpui::Pixels>>>,
    slider_drag: Option<(Mode, u8)>,
}

fn scalar(transport: &mut Transport, command: u8) -> Result<u8> {
    let data = transport.execute(protocol::request(command, None))?;
    ensure!(data.len() >= 5, "OutData 需要 5 字节");
    Ok(data[4])
}

fn read(transport: &mut Transport, settings: &Settings) -> Result<Snapshot> {
    let mode = Mode::from_byte(scalar(transport, protocol::MODE)?)?;
    let custom = scalar(transport, 23)? == 1;
    let fan_mode = if custom { Mode::Performance } else { mode };
    let speed = fan_mode.initial_speed(
        scalar(transport, protocol::SPEED)?,
        settings.speed(fan_mode)?,
    );
    let boost = scalar(transport, protocol::SWITCH)? == 1;
    let (cpu, gpu) = protocol::rpm(&transport.execute(protocol::request(protocol::RPM, None))?)?;
    Ok(Snapshot {
        mode,
        custom,
        speed,
        boost,
        cpu,
        gpu,
    })
}

fn worker(
    commands: Receiver<Command>,
    updates: Sender<Result<Snapshot, String>>,
    path: std::path::PathBuf,
    legacy: Option<std::path::PathBuf>,
) {
    let mut transport = match Transport::new() {
        Ok(t) => Some(t),
        Err(e) => {
            let _ = updates.send(Err(format!("{e:#}")));
            None
        }
    };
    let mut settings = match Settings::open_with_legacy(path, legacy.as_deref()) {
        Ok(s) => s,
        Err(e) => {
            let _ = updates.send(Err(e.to_string()));
            return;
        }
    };
    let initial = transport.as_mut().map(|t| read(t, &settings));
    let mut previous = initial.as_ref().and_then(|r| r.as_ref().ok()).cloned();
    if let Some(initial) = initial {
        let _ = updates.send(initial.map_err(|e| format!("{e:#}")));
    }
    loop {
        let command = match commands.recv_timeout(Duration::from_secs(10)) {
            Ok(command) => command,
            Err(mpsc::RecvTimeoutError::Timeout) => Command::Refresh,
            Err(mpsc::RecvTimeoutError::Disconnected) => return,
        };
        let result = (|| -> Result<Snapshot> {
            if transport.is_none() {
                transport = Some(Transport::new()?);
            }
            let transport = transport.as_mut().unwrap();
            let mut state = read(transport, &settings)?;
            // MainWindow's SystemPerMode event saves the outgoing fan setting
            // and restores the saved value for the incoming mode.
            if let Some(old) = &previous
                && (old.mode != state.mode || old.custom != state.custom)
            {
                settings.save(old.fan_mode(), old.speed)?;
                let saved = settings.speed(state.fan_mode())?;
                transport.execute(protocol::request(protocol::SPEED, Some(saved)))?;
                state = read(transport, &settings)?;
            }
            match command {
                Command::Refresh => {}
                Command::SetMode(mode) => {
                    ensure!(scalar(transport, 19)? != 1, "Type-C 供电下无法切换性能模式");
                    settings.save(state.fan_mode(), state.speed)?;
                    transport.execute(protocol::request(protocol::MODE, Some(mode.byte())))?;
                    if state.custom {
                        transport.execute(protocol::request(23, Some(0)))?;
                    }
                    let updated = read(transport, &settings)?;
                    ensure!(
                        updated.mode == mode && !updated.custom,
                        "固件未切换到请求的模式"
                    );
                    transport.execute(protocol::request(
                        protocol::SPEED,
                        Some(settings.speed(mode)?),
                    ))?;
                }
                Command::Toggle(value) => {
                    transport.execute(protocol::request(protocol::SWITCH, Some(value as u8)))?;
                }
                Command::Adjust(delta) => {
                    let speed = state.speed as i16 + delta as i16;
                    let (min, max) = state.fan_mode().limits();
                    if (min as i16..=max as i16).contains(&speed) {
                        transport.execute(protocol::request(protocol::SWITCH, Some(1)))?;
                        transport.execute(protocol::request(protocol::SPEED, Some(speed as u8)))?;
                        settings.save(state.fan_mode(), speed as u8)?;
                    }
                }
                Command::SetSpeed(speed) => {
                    let (min, max) = state.fan_mode().limits();
                    ensure!((min..=max).contains(&speed), "目标转速超出当前模式范围");
                    transport.execute(protocol::request(protocol::SWITCH, Some(1)))?;
                    transport.execute(protocol::request(protocol::SPEED, Some(speed)))?;
                    settings.save(state.fan_mode(), speed)?;
                }
            }
            read(transport, &settings)
        })();
        if let Ok(state) = &result {
            previous = Some(state.clone());
        }
        if updates.send(result.map_err(|e| format!("{e:#}"))).is_err() {
            return;
        }
    }
}

impl View {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        cx.observe_window_appearance(window, |_, _, cx| cx.notify())
            .detach();
        let system_font = ui::system_font();
        let focus = cx.focus_handle();
        let slider_focus = cx.focus_handle().tab_stop(true);
        let slider_bounds = std::rc::Rc::new(std::cell::Cell::new(Bounds::default()));
        window.focus(&focus, cx);
        if let Some(preview) =
            std::env::args().find_map(|arg| arg.strip_prefix("--preview-ui=").map(str::to_owned))
        {
            let options: Vec<_> = preview.split(',').collect();
            let (tx, _) = mpsc::channel();
            let (_, rx) = mpsc::channel();
            let office = options.contains(&"office");
            cx.spawn(async move |view, cx| {
                cx.background_executor()
                    .timer(Duration::from_millis(150))
                    .await;
                let _ = view.update(cx, |_, cx| cx.notify());
            })
            .detach();
            return Self {
                tx,
                rx,
                system_font,
                preview: true,
                focus,
                slider_focus,
                slider_bounds,
                slider_drag: None,
                preferences: Preferences {
                    theme: if options.contains(&"dark") {
                        jiaolong_control::preferences::Theme::Dark
                    } else {
                        jiaolong_control::preferences::Theme::Light
                    },
                    language: if options.contains(&"en") {
                        jiaolong_control::preferences::Language::En
                    } else {
                        jiaolong_control::preferences::Language::ZhCn
                    },
                },
                preferences_path: preferences_path(),
                preference_error: None,
                loading: false,
                pending: false,
                page: if options.contains(&"settings") {
                    ui::Page::Settings
                } else if options.contains(&"features") {
                    ui::Page::Overview
                } else {
                    ui::Page::Fan
                },
                language_menu_open: options.contains(&"menu"),
                state: if options.contains(&"error") {
                    None
                } else {
                    Some(Snapshot {
                        mode: if office { Mode::Quiet } else { Mode::Balanced },
                        custom: false,
                        speed: if office { 27 } else { 43 },
                        boost: !office,
                        cpu: if office { 2680 } else { 4320 },
                        gpu: if office { 2510 } else { 4260 },
                    })
                },
                status: "Preview: firmware interface unavailable".into(),
            };
        }
        let preferences_path = preferences_path();
        let (preferences, preference_error) = match Preferences::load(&preferences_path) {
            Ok(preferences) => (preferences, None),
            Err(error) => (Preferences::default(), Some(format!("{error:#}"))),
        };
        let (tx, commands) = mpsc::channel();
        let (updates, rx) = mpsc::channel();
        let legacy = std::env::current_exe()
            .unwrap()
            .with_file_name("AppMachineInfo");
        #[cfg(target_os = "linux")]
        let (path, legacy) = (
            preferences_path.with_file_name("AppMachineInfo"),
            Some(legacy),
        );
        #[cfg(not(target_os = "linux"))]
        let (path, legacy) = (legacy, None);
        std::thread::spawn(move || worker(commands, updates, path, legacy));
        cx.spawn(async move |view, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(100))
                    .await;
                if view
                    .update(cx, |view, cx| {
                        while let Ok(update) = view.rx.try_recv() {
                            view.loading = false;
                            view.pending = false;
                            match update {
                                Ok(state) => {
                                    if view
                                        .slider_drag
                                        .is_some_and(|(mode, _)| mode != state.fan_mode())
                                    {
                                        view.slider_drag = None;
                                    }
                                    view.state = Some(state);
                                    view.status = "已连接".into();
                                }
                                Err(error) => {
                                    view.status = error;
                                }
                            }
                            cx.notify();
                        }
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
        Self {
            tx,
            rx,
            system_font,
            preview: false,
            focus,
            slider_focus,
            slider_bounds,
            slider_drag: None,
            preferences,
            preferences_path,
            preference_error,
            loading: true,
            pending: false,
            page: ui::Page::Fan,
            language_menu_open: false,
            state: None,
            status: "正在读取风扇状态…".into(),
        }
    }
    fn send(&mut self, command: Command, cx: &mut Context<Self>) {
        self.slider_drag = None;
        if self.preview {
            if let Some(state) = &mut self.state {
                match command {
                    Command::Refresh => {}
                    Command::SetMode(mode) => {
                        state.mode = mode;
                        state.custom = false;
                        state.speed = match mode {
                            Mode::Quiet => 27,
                            Mode::Balanced => 43,
                            Mode::Performance => 53,
                        };
                    }
                    Command::Toggle(value) => state.boost = value,
                    Command::Adjust(delta) => {
                        let speed = state.speed as i16 + delta as i16;
                        let (min, max) = state.fan_mode().limits();
                        if (min as i16..=max as i16).contains(&speed) {
                            state.speed = speed as u8;
                            state.boost = true;
                        }
                    }
                    Command::SetSpeed(speed) => {
                        let (min, max) = state.fan_mode().limits();
                        if (min..=max).contains(&speed) {
                            state.speed = speed;
                            state.boost = true;
                        }
                    }
                }
            }
            cx.notify();
            return;
        }
        self.pending = true;
        match self.tx.send(command) {
            Ok(()) => self.status = "正在读取…".into(),
            Err(_) => {
                self.pending = false;
                self.state = None;
                self.status = "Hardware connection ended".into();
            }
        }
        cx.notify();
    }
}

struct Diagnostics;
impl log::Log for Diagnostics {
    fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
        metadata.level() <= log::Level::Info
    }
    fn log(&self, record: &log::Record<'_>) {
        if self.enabled(record.metadata()) {
            eprintln!("{}: {}", record.level(), record.args());
        }
    }
    fn flush(&self) {}
}

fn main() {
    let _ = log::set_logger(&Diagnostics);
    log::set_max_level(log::LevelFilter::Warn);
    gpui_platform::application()
        .with_assets(ui::Assets)
        .run(|cx: &mut App| {
            cx.on_window_closed(|cx, _| {
                if cx.windows().is_empty() {
                    cx.quit();
                }
            })
            .detach();
            let bounds = Bounds::centered(None, size(px(1008.), px(704.)), cx);
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    titlebar: Some(gpui::TitlebarOptions {
                        title: Some("Jiaolong Control Center".into()),
                        appears_transparent: true,
                        ..Default::default()
                    }),
                    app_id: Some("jiaolong-control".into()),
                    window_min_size: Some(size(px(880.), px(680.))),
                    ..Default::default()
                },
                |window, cx| cx.new(|cx| View::new(window, cx)),
            )
            .unwrap();
            cx.activate(true);
        });
}

#[cfg(all(test, target_os = "linux"))]
mod hardware_tests {
    use super::*;
    use std::{
        io::Write,
        path::PathBuf,
        process::{Command as ProcessCommand, Stdio},
    };

    fn profile(path: &PathBuf, value: &str) {
        let mut child = ProcessCommand::new("sudo")
            .args(["-n", "tee"])
            .arg(path)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(format!("{value}\n").as_bytes())
            .unwrap();
        assert!(child.wait().unwrap().success());
    }
    struct Restore {
        transport: Transport,
        speed: u8,
        boost: u8,
        profile_path: PathBuf,
        profile_value: String,
        settings_path: PathBuf,
    }
    impl Drop for Restore {
        fn drop(&mut self) {
            profile(&self.profile_path, &self.profile_value);
            self.transport
                .execute(protocol::request(protocol::SPEED, Some(self.speed)))
                .unwrap();
            self.transport
                .execute(protocol::request(protocol::SWITCH, Some(self.boost)))
                .unwrap();
            assert_eq!(
                scalar(&mut self.transport, protocol::SPEED).unwrap(),
                self.speed
            );
            assert_eq!(
                scalar(&mut self.transport, protocol::SWITCH).unwrap(),
                self.boost
            );
            let _ = std::fs::remove_file(&self.settings_path);
        }
    }
    struct Worker {
        tx: Option<Sender<Command>>,
        handle: Option<std::thread::JoinHandle<()>>,
    }
    impl Drop for Worker {
        fn drop(&mut self) {
            self.tx.take();
            self.handle.take().unwrap().join().unwrap();
        }
    }

    #[test]
    #[ignore = "实机风扇写入与电源模式切换；结束时恢复原状态"]
    fn hardware_controller_roundtrip() {
        let profile_path = std::fs::read_dir("/sys/class/platform-profile")
            .unwrap()
            .map(|entry| entry.unwrap().path().join("profile"))
            .find(|path| {
                path.canonicalize()
                    .unwrap()
                    .to_string_lossy()
                    .contains("B60BFB48")
            })
            .unwrap();
        let profile_value = std::fs::read_to_string(&profile_path)
            .unwrap()
            .trim()
            .to_owned();
        let mut transport = Transport::new().unwrap();
        let speed = scalar(&mut transport, protocol::SPEED).unwrap();
        let boost = scalar(&mut transport, protocol::SWITCH).unwrap();
        let settings_path =
            std::env::temp_dir().join(format!("jiaolong-worker-{}", std::process::id()));
        let restore = Restore {
            transport,
            speed,
            boost,
            profile_path,
            profile_value,
            settings_path: settings_path.clone(),
        };
        let (tx, commands) = mpsc::channel();
        let (updates, rx) = mpsc::channel();
        let handle = std::thread::spawn(move || worker(commands, updates, settings_path, None));
        let worker = Worker {
            tx: Some(tx),
            handle: Some(handle),
        };
        let receive = || rx.recv_timeout(Duration::from_secs(5)).unwrap().unwrap();
        let send = |command| {
            worker.tx.as_ref().unwrap().send(command).unwrap();
            receive()
        };
        let initial = receive();
        assert_eq!(initial.mode, Mode::Balanced);
        assert!(!initial.custom);
        assert!(initial.cpu > 0 && initial.gpu > 0);
        let delta = if initial.speed < 50 { 1 } else { -1 };
        let adjusted = send(Command::Adjust(delta));
        assert_eq!(adjusted.speed as i16, initial.speed as i16 + delta as i16);
        assert!(adjusted.boost);
        assert_eq!(
            Settings::open(restore.settings_path.clone())
                .unwrap()
                .speed(Mode::Balanced)
                .unwrap(),
            adjusted.speed
        );
        assert_eq!(send(Command::Adjust(-delta)).speed, initial.speed);
        assert!(!send(Command::Toggle(false)).boost);
        assert!(send(Command::Toggle(true)).boost);
        let started = std::time::Instant::now();
        let quiet = send(Command::SetMode(Mode::Quiet));
        assert!(started.elapsed() < Duration::from_secs(5));
        assert_eq!(quiet.mode, Mode::Quiet);
        assert_eq!(quiet.speed, 27);
        let restored = send(Command::SetMode(initial.mode));
        assert_eq!(restored.mode, initial.mode);
        assert_eq!(restored.speed, initial.speed);
        drop(worker);
        drop(restore);
    }
}
