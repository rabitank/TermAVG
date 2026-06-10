use std::num::NonZeroU32;
use std::sync::Arc;
use std::sync::mpsc::Receiver;
use std::time::{self, Duration};

use anyhow::Context;
use chrono::{FixedOffset, Utc};
use ratatui::Terminal;
use ratatui_wgpu::{Builder, Dimensions, Font, WgpuBackend};
use tmj_app::app::App;
use tmj_app::audio::AUDIOM;
use tmj_app::art::theme::DARK_GRAY;
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
// static FONT_DATA: &[u8] = include_bytes!("./MapleMono-NF-CN-Regular.ttf");
// static FONT_DATA: &[u8] = include_bytes!("./SarasaTermCL-Regular.ttf");
static FONT_DATA: &[u8] = include_bytes!("./Ligaconsolaslxgw.ttf");
//
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

//// 细胞尺寸 & 字号计算 ////
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

//// 应用状态 ////
struct TuiApp {
    app: App<WgpuBackend<'static, 'static>>,
    last_tick: std::time::Instant,
    receiver: Receiver<GameEvent>,
}

//// winit ApplicationHandler ////
struct AppHandler {
    tui_app: Option<TuiApp>,
    window: Option<Arc<Window>>,
    modifiers: ModifiersState,
}

impl ApplicationHandler for AppHandler {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        tracing::info!("creating window and wgpu backend");

        // 创建全屏窗口
        let window = Arc::new(
            event_loop
                .create_window(WindowAttributes::default())
                .unwrap(),
        );
        window.set_fullscreen(Some(winit::window::Fullscreen::Borderless(None)));

        // 初始化事件系统（NoopProvider：事件来源是 winit，不是 crossterm）
        let (looper, receiver) = EventLooper::new_with_provider(256, Box::new(NoopProvider));
        EventSender::init(looper.sender.clone());
        EventManager::init(looper);
        EventManager::with_looper(|l| l.cool_down(Duration::from_millis(100)));

        // 计算字号和细胞尺寸
        let (font_size, cell_w, cell_h) = eval_cell_size();
        let cols = SETTING.resolution.0;
        let rows = SETTING.resolution.1;

        // 创建 ratatui-wgpu 后端
        let font = Font::new(FONT_DATA).unwrap();
        let pix_w = (cols as u32) * cell_w;
        let pix_h = (rows as u32) * cell_h;
        let dims = Dimensions {
            width: NonZeroU32::new(pix_w).unwrap(),
            height: NonZeroU32::new(pix_h).unwrap(),
        };
        tracing::info!("pixmap {pix_w}x{pix_h}");
        let backend = pollster::block_on(
            Builder::from_font(font)
                .with_font_size_px(font_size)
                .with_dimensions(dims)
                .with_bg_color(DARK_GRAY)
                .build_with_target(window.clone()),
        )
        .unwrap();

        let terminal = Terminal::new(backend).unwrap();
        let app = App::new(terminal);
        let last_tick = time::Instant::now();

        self.tui_app = Some(TuiApp {
            app,
            last_tick,
            receiver,
        });
        self.window = Some(window);

        tracing::info!("wgpu backend ready");
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        match event {
            // 关闭请求
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }

            // 键盘事件
            WindowEvent::KeyboardInput {
                event,
                is_synthetic: false,
                ..
            } => {
                if let Some(game_event) = convert_key_event(&event, &self.modifiers) {
                    let _ = EventSender::sender_event(game_event);
                }
            }

            // 修饰键状态变化
            WindowEvent::ModifiersChanged(mods) => {
                self.modifiers = mods.state();
            }

            // 鼠标按键（坐标固定 0,0，游戏只需按键类型）
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
                if self.modifiers.shift_key() { mods |= KeyModifiers::SHIFT; }
                if self.modifiers.control_key() { mods |= KeyModifiers::CONTROL; }
                if self.modifiers.alt_key() { mods |= KeyModifiers::ALT; }
                if self.modifiers.super_key() { mods |= KeyModifiers::SUPER; }
                let ct_event = crossterm::event::Event::Mouse(MouseEvent {
                    kind,
                    column: 0,
                    row: 0,
                    modifiers: mods,
                });
                let _ = EventSender::sender_event(convert_crossterm_event(ct_event));
            }

            // 窗口大小变化
            WindowEvent::Resized(size) => {
                if let Some(tui) = &mut self.tui_app {
                    tui.app.terminal.backend_mut().resize(size.width, size.height);
                }
            }

            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let Some(tui_app) = &mut self.tui_app else {
            return;
        };
        let Some(window) = &self.window else {
            return;
        };

        let tick = tui_app.last_tick.elapsed();
        tui_app.last_tick = time::Instant::now();

        ////// 事件阶段 //////
        {
            let _span = info_span!("events");
            EventManager::with_looper(|l| {
                if !l.check_is_warmup() {
                    l.drain_buffer(&tui_app.receiver);
                }
            });
        }

        ////// 游戏逻辑阶段 //////
        {
            let mut game = tui_app.app.game.borrow_mut();
            while let Ok(event) = tui_app.receiver.try_recv() {
                if !game.handle_event(&event).context("event").is_ok_and(|v| v) {
                    return;
                }
            }
            game.handle_tick(tick);
            for cmd in CmdBuffer::take_commands() {
                let _ = game.handle_cmd(&cmd);
            }
        }

        ////// 渲染阶段 //////
        {
            let game = tui_app.app.game.borrow_mut();
            let _ = tui_app.app.terminal.draw(|f| game.draw(f));
        }

        ////// 清理阶段：退出检测 //////
        if tui_app.app.game.borrow().game_flow.borrow().is_ready_quit() {
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                AUDIOM.with(|a| {
                    let _ = a.replace(None);
                });
            }));
            EventManager::with_looper(|l| l.stop());
            event_loop.exit();
            return;
        }

        // 请求下一帧
        window.request_redraw();
    }
}

fn main() {
    init_log();

    let event_loop = EventLoop::new().unwrap();
    let mut handler = AppHandler {
        tui_app: None,
        window: None,
        modifiers: ModifiersState::default(),
    };

    let _ = event_loop.run_app(&mut handler);
}
