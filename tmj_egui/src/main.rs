use std::sync::mpsc::Receiver;
use std::time::{self, Duration};

use anyhow::Context;
use chrono::{FixedOffset, Utc};
use eframe::egui::{self, ViewportBuilder, ViewportCommand};
use ratatui::Terminal;
use soft_ratatui::{CosmicText, SoftBackend};
use tmj_app::app::App;
use tmj_app::audio::AUDIOM;
use tmj_app::setting::SETTING;
use tmj_core::command::CmdBuffer;
use tmj_core::event::handler::EventDispatcher;
use tmj_core::event::looper::EventLooper;
use tmj_core::event::provider::{NoopProvider, convert_crossterm_event};
use tmj_core::event::sender::EventSender;
use tmj_core::event::{EventManager, GameEvent};
use tmj_core::pathes;
use tmj_core::pathes::PathResolver;
use tracing::{info_span, instrument};
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::time::FormatTime;
use tracing_subscriber::layer::SubscriberExt;

static FONT_DATA: &[u8] = include_bytes!("./MapleMono-NF-CN-Regular.ttf");

struct ChinaLocalTime;
impl FormatTime for ChinaLocalTime {
    fn format_time(&self, w: &mut Writer<'_>) -> std::fmt::Result {
        let tz = FixedOffset::east_opt(8 * 3600).unwrap();
        let now = Utc::now().with_timezone(&tz);
        write!(w, "{}", now.format("%m-%d %H:%M:%S%.3f"))
    }
}

fn screen_size() -> (f32, f32, f32) {
    if let Ok(displays) = display_info::DisplayInfo::all() {
        if let Some(d) = displays.iter().find(|d| d.is_primary).or(displays.first()) {
            return (d.width as f32, d.height as f32, d.scale_factor.max(1.0));
        }
    }
    (1920.0, 1080.0, 1.0)
}

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
    let tracy_layer = tracing_tracy::TracyLayer::default();
    // 组合成一个订阅器
    let subscriber = tracing_subscriber::Registry::default()
        .with(log_txt_layer)
        .with(tracy_layer);

    // 一次性设置全局默认订阅器
    tracing::subscriber::set_global_default(subscriber).unwrap();
}

fn eval_cell_size() -> (u32, u32, u32) {
    let (scr_w, scr_h, dpi) = screen_size();
    let rows = SETTING.resolution.1;
    let dpi_f = dpi.max(1.0);
    let raw = scr_h / dpi_f / rows as f32;
    let cell_h = (((raw / 2.0).floor() * 2.0) as u32).max(12).min(24);
    let cell_w = cell_h / 2;
    let font_size = cell_h;

    tracing::info!(
        "screen {scr_w}x{scr_h} dpi {dpi_f} cell {cell_w}x{cell_h} font_size {font_size} "
    );

    (font_size, cell_w, cell_h)
}

fn main() -> eframe::Result {
    init_log();
    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_fullscreen(true)
            .with_decorations(false),
        ..Default::default()
    };

    let my_app = MyApp::new();

    eframe::run_native(
        "TermAVG",
        options,
        Box::new(|_cc| {
            // This gives us image support:
            Ok(Box::new(my_app))
        }),
    )
}

struct MyApp {
    pub app: App<SoftBackend<CosmicText>>,
    pub last_tick: std::time::Instant,
    pub receiver: Receiver<GameEvent>,
    pub text_ref: Option<egui::TextureHandle>,
}

impl MyApp {
    fn new() -> Self {
        let (looper, receiver) = EventLooper::new_with_provider(256, Box::new(NoopProvider));
        EventSender::init(looper.sender.clone());
        EventManager::init(looper);
        EventManager::with_looper(|l| l.cool_down(Duration::from_millis(100)));

        let (font_size, cell_w, cell_h) = eval_cell_size();
        let mut backend = SoftBackend::<CosmicText>::new(
            SETTING.resolution.0,
            SETTING.resolution.1,
            font_size as i32,
            &FONT_DATA,
        );
        backend.char_width = cell_w as usize;
        backend.char_height = cell_h as usize;
        // backend.resize(SETTING.resolution.0, SETTING.resolution.1);

        let area = ratatui::layout::Rect::new(0, 0, SETTING.resolution.0, SETTING.resolution.1);
        backend.buffer.set_style(
            area,
            ratatui::style::Style::new().bg(ratatui::style::Color::Black),
        );
        backend.redraw();

        let pix_w = backend.get_pixmap_width() as f32;
        let pix_h = backend.get_pixmap_height() as f32;
        tracing::info!("pixmap {pix_w}x{pix_h}");

        let terminal = Terminal::new(backend).unwrap();
        let app = App::new(terminal);
        let last_tick = time::Instant::now();

        Self {
            app,
            last_tick,
            receiver,
            text_ref: None,
        }
    }
}

impl eframe::App for MyApp {

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let tick = self.last_tick.elapsed();
        self.last_tick = time::Instant::now();
        ctx.set_visuals(egui::Visuals::dark());

        let events = ctx.input(|i| i.events.clone());
        for ev in &events {
            if let Ok(t) = terminput_egui::to_terminput(ev.clone()) {
                if let Ok(c) = terminput_crossterm::to_crossterm(t) {
                    let _ = EventSender::sender_event(convert_crossterm_event(c));
                }
            }
        }

        EventManager::with_looper(|l| {
            if !l.check_is_warmup() {
                l.drain_buffer(&self.receiver);
            }
        });

        {
            let mut game = self.app.game.borrow_mut();
            while let Ok(event) = self.receiver.try_recv() {
                if !game.handle_event(&event).context("event").is_ok_and(|v| v) {
                    return;
                }
            }
            game.handle_tick(tick);
            for cmd in CmdBuffer::take_commands() {
                let _ = game.handle_cmd(&cmd);
            }
            self.app.terminal.clear();
            self.app.terminal.draw(|f| game.draw(f)).ok();
        }
        let colorik = egui::ColorImage::from_rgb(
            [
                self.app.terminal.backend().get_pixmap_width(),
                self.app.terminal.backend().get_pixmap_height(),
            ],
            self.app.terminal.backend().get_pixmap_data(),
        );

        // terminal.draw(draw).expect("failed to draw frame");
        if let Some(text_handle) = &mut self.text_ref {
            text_handle.set(colorik, Default::default());
        } else {
            let texture = ctx.load_texture(
                "game render",   // texture ID (can be anything)
                colorik.clone(), // your ColorImage
                Default::default(),
            );
            self.text_ref = Some(texture.clone());
        };

        let texture = self.text_ref.clone().unwrap();

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::both().show(ui, |ui| {
                // ui.add(colorik);
                println!("{:?}", texture.id());
                ui.image((texture.id(), texture.size_vec2()));
            });
        });

        let frame_budget = Duration::from_millis(16);
        let remaining = frame_budget.saturating_sub(tick);
        ctx.request_repaint();
        // ctx.request_repaint_after(remaining);

        if self.app.game.borrow().game_flow.borrow().is_ready_quit() {
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                AUDIOM.with(|a| {
                    let _ = a.replace(None);
                });
            }));
            EventManager::with_looper(|l| l.stop());
            ctx.send_viewport_cmd(ViewportCommand::Close);
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {}
}

