use std::num::NonZeroU32;
use std::sync::Arc;
use std::sync::mpsc::Receiver;
use std::time::{self, Duration};

use anyhow::Context;
use chrono::{FixedOffset, Utc};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Modifier, Style},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame, Terminal,
};
use ratatui_wgpu::{Builder, Dimensions, Font, WgpuBackend};
use tmj_app::app::App;
use tmj_app::audio::AUDIOM;
use tmj_app::art::theme::{BLACK, DARK_GRAY, LIGHT_GRAY, MID_GRAY, WHITE};
use tmj_app::setting::SETTING;
use tmj_core::command::CmdBuffer;
use tmj_core::event::handler::EventDispatcher;
use tmj_core::event::looper::EventLooper;
use tmj_core::event::provider::{NoopProvider, convert_crossterm_event};
use tmj_core::event::sender::EventSender;
use tmj_core::event::{EventManager, GameEvent};
use tmj_core::pathes::{self, PathResolver};
use tracing::info_span;
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::time::FormatTime;
use tracing_subscriber::layer::SubscriberExt;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, KeyEvent, MouseButton as WinitMouseBtn, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{Key, ModifiersState, NamedKey};
use winit::window::{Window, WindowAttributes};

use crossterm::event::{
    KeyCode, KeyEventKind, KeyEventState, KeyModifiers, MouseButton as CtMouseBtn, MouseEvent,
    MouseEventKind,
};

//// Font data ////
static FONT_DATA: &[u8] = include_bytes!("./Ligaconsolaslxgw.ttf");

//// 窗口大小预设 ////
struct SizePreset {
    name: &'static str,
    desc: &'static str,
    cell_h: u32,
    cell_w: u32,
    font_size: u32,
}

const PRESETS: [SizePreset; 3] = [
    SizePreset { name: "1K",  desc: "1680×938 +padding", cell_h: 14, cell_w: 7,  font_size: 14 },
    SizePreset { name: "2K",  desc: "2160×1206 +padding", cell_h: 18, cell_w: 9,  font_size: 18 },
    SizePreset { name: "4K",  desc: "2640×1474 +padding", cell_h: 22, cell_w: 11, font_size: 22 },
];

//// 东八区时间格式化 ////
struct ChinaLocalTime;
impl FormatTime for ChinaLocalTime {
    fn format_time(&self, w: &mut Writer<'_>) -> std::fmt::Result {
        let tz = FixedOffset::east_opt(8 * 3600).unwrap();
        let now = Utc::now().with_timezone(&tz);
        write!(w, "{}", now.format("%m-%d %H:%M:%S%.3f"))
    }
}

//// 日志初始化 ////
fn init_log() {
    PathResolver::global_init();
    let writer_path = pathes::path("log.txt");
    let _ = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&writer_path);
    let log_txt_layer = tracing_subscriber::fmt::layer()
        .with_timer(ChinaLocalTime)
        .with_writer(move || {
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&writer_path)
                .expect("open log")
        });
    let subscriber = tracing_subscriber::Registry::default().with(log_txt_layer);
    tracing::subscriber::set_global_default(subscriber).unwrap();
}

//// 屏幕信息 ////
fn screen_size() -> (f32, f32, f32) {
    if let Ok(displays) = display_info::DisplayInfo::all() {
        if let Some(d) = displays.iter().find(|d| d.is_primary).or(displays.first()) {
            return (d.width as f32, d.height as f32, d.scale_factor.max(1.0));
        }
    }
    (1920.0, 1080.0, 1.0)
}

//// 细胞尺寸 & 字号计算（基于实际显示器） ////
fn eval_cell_size() -> (u32, u32, u32) {
    let (scr_w, scr_h, dpi) = screen_size();
    let rows = SETTING.resolution.1;
    let dpi_f = dpi.max(1.0);
    let raw = scr_h / dpi_f / rows as f32;
    let cell_h = (((raw / 2.0).floor() * 2.0) as u32).max(12).min(24);
    let cell_w = cell_h / 2;
    let font_size = cell_h;
    tracing::info!(
        "screen {scr_w}x{scr_h} dpi {dpi_f} cell {cell_w}x{cell_h} font_size {font_size}"
    );
    (font_size, cell_w, cell_h)
}

//// winit NamedKey -> crossterm KeyCode ////
fn named_to_keycode(named: NamedKey) -> Option<KeyCode> {
    match named {
        NamedKey::Enter => Some(KeyCode::Enter),
        NamedKey::Tab => Some(KeyCode::Tab),
        NamedKey::Backspace => Some(KeyCode::Backspace),
        NamedKey::Escape => Some(KeyCode::Esc),
        NamedKey::Space => Some(KeyCode::Char(' ')),
        NamedKey::Delete => Some(KeyCode::Delete),
        NamedKey::Home => Some(KeyCode::Home),
        NamedKey::End => Some(KeyCode::End),
        NamedKey::PageUp => Some(KeyCode::PageUp),
        NamedKey::PageDown => Some(KeyCode::PageDown),
        NamedKey::Insert => Some(KeyCode::Insert),
        NamedKey::ArrowUp => Some(KeyCode::Up),
        NamedKey::ArrowDown => Some(KeyCode::Down),
        NamedKey::ArrowLeft => Some(KeyCode::Left),
        NamedKey::ArrowRight => Some(KeyCode::Right),
        NamedKey::F1 => Some(KeyCode::F(1)),
        NamedKey::F2 => Some(KeyCode::F(2)),
        NamedKey::F3 => Some(KeyCode::F(3)),
        NamedKey::F4 => Some(KeyCode::F(4)),
        NamedKey::F5 => Some(KeyCode::F(5)),
        NamedKey::F6 => Some(KeyCode::F(6)),
        NamedKey::F7 => Some(KeyCode::F(7)),
        NamedKey::F8 => Some(KeyCode::F(8)),
        NamedKey::F9 => Some(KeyCode::F(9)),
        NamedKey::F10 => Some(KeyCode::F(10)),
        NamedKey::F11 => Some(KeyCode::F(11)),
        NamedKey::F12 => Some(KeyCode::F(12)),
        _ => None,
    }
}

//// winit KeyEvent -> Option<GameEvent> ////
fn convert_key_event(event: &KeyEvent, mods: &ModifiersState) -> Option<GameEvent> {
    let key_code = match &event.logical_key {
        Key::Character(s) => {
            let s = s.as_str();
            if s.len() == 1 {
                s.chars().next().map(KeyCode::Char)
            } else {
                None
            }
        }
        Key::Named(named) => named_to_keycode(*named),
        _ => None,
    };

    key_code.map(|code| {
        let mut crossterm_mods = KeyModifiers::NONE;
        if mods.shift_key() {
            crossterm_mods |= KeyModifiers::SHIFT;
        }
        if mods.control_key() {
            crossterm_mods |= KeyModifiers::CONTROL;
        }
        if mods.alt_key() {
            crossterm_mods |= KeyModifiers::ALT;
        }
        if mods.super_key() {
            crossterm_mods |= KeyModifiers::SUPER;
        }
        let kind = match event.state {
            ElementState::Pressed if event.repeat => KeyEventKind::Repeat,
            ElementState::Pressed => KeyEventKind::Press,
            ElementState::Released => KeyEventKind::Release,
        };
        let ct_event = crossterm::event::Event::Key(crossterm::event::KeyEvent {
            code,
            modifiers: crossterm_mods,
            kind,
            state: KeyEventState::NONE,
        });
        convert_crossterm_event(ct_event)
    })
}

//// 启动器状态 ////
struct LauncherState {
    window: Arc<Window>,
    term: Terminal<WgpuBackend<'static, 'static>>,
    selected: usize,
    fullscreen: bool,
    size_preset: usize,
    should_start: bool,
    should_exit: bool,
}

impl LauncherState {
    fn menu_labels(&self) -> Vec<String> {
        vec![
            format!("显示模式    {}", if self.fullscreen { "全屏" } else { "窗口" }),
            format!(
                "窗口大小    {} （{}）",
                PRESETS[self.size_preset].name,
                PRESETS[self.size_preset].desc
            ),
            "开始游戏".into(),
            "退出".into(),
        ]
    }

    fn item_count(&self) -> usize {
        4
    }
}

//// 游戏状态 ////
struct GameState {
    app: App<WgpuBackend<'static, 'static>>,
    window: Arc<Window>,
    receiver: Receiver<GameEvent>,
    last_tick: std::time::Instant,
}

//// 阶段枚举 ////
enum AppPhase {
    Launch(LauncherState),
    Game(GameState),
}

//// 启动器 UI ////
fn draw_launcher(frame: &mut Frame, labels: &[String], selected: usize) {
    let area = frame.area();
    let border_style = Style::new().fg(MID_GRAY);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints::<&[Constraint]>(&[
            Constraint::Length(4),
            Constraint::Length((labels.len() * 2 + 1) as u16),
            Constraint::Length(3),
        ])
        .split(area);

    // 标题
    let title = Paragraph::new("TerminalLove")
        .style(Style::new().fg(WHITE).add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::ALL).border_style(border_style));
    frame.render_widget(title, chunks[0]);

    // 菜单列表
    let list_items: Vec<ListItem> = labels
        .iter()
        .map(|s| ListItem::new(s.as_str()).style(Style::new().fg(LIGHT_GRAY)))
        .collect();
    let list = List::new(list_items)
        .block(Block::default().borders(Borders::ALL).border_style(border_style))
        .highlight_style(Style::new().fg(BLACK).bg(WHITE))
        .highlight_symbol("> ");
    let mut list_state = ratatui::widgets::ListState::default().with_selected(Some(selected));
    frame.render_stateful_widget(list, chunks[1], &mut list_state);

    // 提示
    let hint = Paragraph::new("Arrow/Space/Enter/Esc")
        .style(Style::new().fg(MID_GRAY));
    frame.render_widget(hint, chunks[2]);
}

//// 创建 WgpuBackend + Terminal 辅助 ////
fn create_backend_and_terminal(
    window: &Arc<Window>,
    font_size: u32,
    cell_w: u32,
    cell_h: u32,
    cols: u32,
    rows: u32,
) -> Terminal<WgpuBackend<'static, 'static>> {
    let font = Font::new(FONT_DATA).unwrap();
    let pix_w = cols * cell_w;
    let pix_h = rows * cell_h;
    let dims = Dimensions {
        width: NonZeroU32::new(pix_w).unwrap(),
        height: NonZeroU32::new(pix_h).unwrap(),
    };
    let backend = pollster::block_on(
        Builder::from_font(font)
            .with_font_size_px(font_size)
            .with_dimensions(dims)
            .with_bg_color(DARK_GRAY)
            .build_with_target(window.clone()),
    )
    .unwrap();
    Terminal::new(backend).unwrap()
}

//// winit ApplicationHandler ////
struct AppHandler {
    phase: Option<AppPhase>,
    modifiers: ModifiersState,
}

impl AppHandler {
    /// 从启动器过渡到游戏
    fn init_game(&mut self, event_loop: &ActiveEventLoop, fullscreen: bool, size_preset: usize) {
        let cols = SETTING.resolution.0 as u32;
        let rows = SETTING.resolution.1 as u32;

        let (window, font_size, cell_w, cell_h) = if fullscreen {
            // 全屏：基于实际显示器计算
            let (font_size, cell_w, cell_h) = eval_cell_size();
            let window = Arc::new(
                event_loop
                    .create_window(WindowAttributes::default())
                    .unwrap(),
            );
            window.set_fullscreen(Some(winit::window::Fullscreen::Borderless(None)));
            (window, font_size, cell_w, cell_h)
        } else {
            // 窗口化：使用预设尺寸
            let preset = &PRESETS[size_preset];
            let inner_w = cols * preset.cell_w;
            let inner_h = rows * preset.cell_h;
            let window = Arc::new(
                event_loop
                    .create_window(
                        WindowAttributes::default()
                            .with_inner_size(winit::dpi::LogicalSize::new(
                                inner_w as f64,
                                inner_h as f64,
                            ))
                            .with_resizable(false)
                            .with_title("TerminalLove"),
                    )
                    .unwrap(),
            );
            (window, preset.font_size, preset.cell_w, preset.cell_h)
        };

        // 初始化事件系统
        let (looper, receiver) = EventLooper::new_with_provider(256, Box::new(NoopProvider));
        EventSender::init(looper.sender.clone());
        EventManager::init(looper);
        EventManager::with_looper(|l| l.cool_down(Duration::from_millis(100)));

        let terminal = create_backend_and_terminal(&window, font_size, cell_w, cell_h, cols, rows);
        let app = App::new(terminal);

        self.phase = Some(AppPhase::Game(GameState {
            app,
            window,
            receiver,
            last_tick: time::Instant::now(),
        }));
    }

    fn handle_launch_key(launch: &mut LauncherState, key: Key<&str>) {
        let count = launch.item_count();
        match key {
            Key::Named(NamedKey::ArrowDown) => {
                launch.selected = (launch.selected + 1) % count;
            }
            Key::Named(NamedKey::ArrowUp) => {
                launch.selected = launch.selected.checked_sub(1).unwrap_or(count - 1);
            }
            Key::Named(NamedKey::Enter) => match launch.selected {
                0 => launch.fullscreen = !launch.fullscreen,
                1 => launch.size_preset = (launch.size_preset + 1) % PRESETS.len(),
                2 => launch.should_start = true,
                _ => launch.should_exit = true,
            },
            Key::Character(" ") => match launch.selected {
                0 => launch.fullscreen = !launch.fullscreen,
                1 => launch.size_preset = (launch.size_preset + 1) % PRESETS.len(),
                _ => {}
            },
            Key::Named(NamedKey::Escape) => launch.should_exit = true,
            _ => {}
        }
    }
}

impl ApplicationHandler for AppHandler {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        tracing::info!("creating launcher window");

        // 固定小窗口：50×18 细胞 @ font_size=16 → 400×288 内部分辨率
        let launcher_cols: u32 = 50;
        let launcher_rows: u32 = 18;
        let launcher_font_size: u32 = 16;
        let launcher_cell_w: u32 = 8;
        let launcher_cell_h: u32 = 16;

        let window = Arc::new(
            event_loop
                .create_window(
                    WindowAttributes::default()
                        .with_inner_size(winit::dpi::LogicalSize::new(
                            (launcher_cols * launcher_cell_w) as f64,
                            (launcher_rows * launcher_cell_h) as f64,
                        ))
                        .with_resizable(false)
                        .with_title("TerminalLove"),
                )
                .unwrap(),
        );

        let term = create_backend_and_terminal(
            &window,
            launcher_font_size,
            launcher_cell_w,
            launcher_cell_h,
            launcher_cols,
            launcher_rows,
        );

        self.phase = Some(AppPhase::Launch(LauncherState {
            window,
            term,
            selected: 0,
            fullscreen: true,
            size_preset: 1,
            should_start: false,
            should_exit: false,
        }));
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match &mut self.phase {
            Some(AppPhase::Launch(launch)) => match event {
                WindowEvent::CloseRequested => {
                    launch.should_exit = true;
                }
                WindowEvent::KeyboardInput {
                    event,
                    is_synthetic: false,
                    ..
                } => {
                    if event.state != ElementState::Released {
                        return;
                    }
                    Self::handle_launch_key(launch, event.logical_key.as_ref());
                }
                _ => {}
            },

            Some(AppPhase::Game(game)) => match event {
                WindowEvent::CloseRequested => {
                    event_loop.exit();
                }
                WindowEvent::KeyboardInput {
                    event,
                    is_synthetic: false,
                    ..
                } => {
                    if let Some(game_event) = convert_key_event(&event, &self.modifiers) {
                        let _ = EventSender::sender_event(game_event);
                    }
                }
                WindowEvent::ModifiersChanged(mods) => {
                    self.modifiers = mods.state();
                }
                WindowEvent::MouseInput {
                    state, button, ..
                } => {
                    let ct_button = match button {
                        WinitMouseBtn::Left => CtMouseBtn::Left,
                        WinitMouseBtn::Right => CtMouseBtn::Right,
                        WinitMouseBtn::Middle => CtMouseBtn::Middle,
                        _ => return,
                    };
                    let kind = match state {
                        ElementState::Pressed => MouseEventKind::Down(ct_button),
                        ElementState::Released => MouseEventKind::Up(ct_button),
                    };
                    let mut mods = KeyModifiers::NONE;
                    if self.modifiers.shift_key() {
                        mods |= KeyModifiers::SHIFT;
                    }
                    if self.modifiers.control_key() {
                        mods |= KeyModifiers::CONTROL;
                    }
                    if self.modifiers.alt_key() {
                        mods |= KeyModifiers::ALT;
                    }
                    if self.modifiers.super_key() {
                        mods |= KeyModifiers::SUPER;
                    }
                    let ct_event = crossterm::event::Event::Mouse(MouseEvent {
                        kind,
                        column: 0,
                        row: 0,
                        modifiers: mods,
                    });
                    let _ = EventSender::sender_event(convert_crossterm_event(ct_event));
                }
                WindowEvent::Resized(size) => {
                    game.app.terminal.backend_mut().resize(size.width, size.height);
                }
                WindowEvent::Focused(true) => {
                    game.window.request_redraw();
                }
                _ => {}
            },

            None => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        match &mut self.phase {
            Some(AppPhase::Launch(launch)) => {
                if launch.should_exit {
                    event_loop.exit();
                    return;
                }
                if launch.should_start {
                    let fullscreen = launch.fullscreen;
                    let size_preset = launch.size_preset;
                    self.phase = None;
                    self.init_game(event_loop, fullscreen, size_preset);
                    return;
                }

                let labels = launch.menu_labels();
                let selected = launch.selected;
                let _ = launch.term.draw(|f| draw_launcher(f, &labels, selected));

                launch.window.request_redraw();
            }

            Some(AppPhase::Game(game)) => {
                let tick = game.last_tick.elapsed();
                game.last_tick = time::Instant::now();

                // 事件阶段
                {
                    let _span = info_span!("events");
                    EventManager::with_looper(|l| {
                        if !l.check_is_warmup() {
                            l.drain_buffer(&game.receiver);
                        }
                    });
                }

                // 游戏逻辑阶段
                {
                    let mut g = game.app.game.borrow_mut();
                    while let Ok(event) = game.receiver.try_recv() {
                        if !g.handle_event(&event).context("event").is_ok_and(|v| v) {
                            return;
                        }
                    }
                    g.handle_tick(tick);
                    for cmd in CmdBuffer::take_commands() {
                        let _ = g.handle_cmd(&cmd);
                    }
                }

                // 渲染阶段
                {
                    let g = game.app.game.borrow_mut();
                    let _ = game.app.terminal.draw(|f| g.draw(f));
                }

                // 退出检测
                if game.app.game.borrow().game_flow.borrow().is_ready_quit() {
                    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        AUDIOM.with(|a| {
                            let _ = a.replace(None);
                        });
                    }));
                    EventManager::with_looper(|l| l.stop());
                    event_loop.exit();
                    return;
                }

                game.window.request_redraw();
            }

            None => {}
        }
    }
}

fn main() {
    init_log();

    let event_loop = EventLoop::new().unwrap();
    let mut handler = AppHandler {
        phase: None,
        modifiers: ModifiersState::default(),
    };

    let _ = event_loop.run_app(&mut handler);
}
