use anyhow::anyhow;
use chrono::{FixedOffset, Utc};
use ratatui::crossterm::event::{
    DisableMouseCapture, EnableMouseCapture, KeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
    poll, read,
};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen, enable_raw_mode};
use std::error::Error;
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::atomic::Ordering;
use std::time::Duration;
use tmj_app::app::App;
use tmj_app::audio::AUDIOM;
use tmj_core::event::EventManager;
use tmj_core::event::looper::EventLooper;
use tmj_core::event::provider;
use tmj_core::event::sender::EventSender;
use tmj_core::pathes;
use tmj_core::pathes::PathResolver;
use tracing::info;
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::time::FormatTime;

const FRAME_DURATION: u8 = (1000 / 60) as u8;

struct ChinaLocalTime;

impl FormatTime for ChinaLocalTime {
    fn format_time(&self, w: &mut Writer<'_>) -> std::fmt::Result {
        let china_tz = FixedOffset::east_opt(8 * 3600).expect("valid china timezone offset");
        let now = Utc::now().with_timezone(&china_tz);
        write!(w, "{}", now.format("%m-%d %H:%M:%S%.3f"))
    }
}

fn init_term() -> ratatui::Terminal<ratatui::prelude::CrosstermBackend<std::io::Stdout>> {
    let _ = enable_raw_mode();
    let mut stdout = std::io::stdout();
    let _ = execute!(stdout, EnterAlternateScreen, EnableMouseCapture);
    // 查询终端是否支持 CSI u（yazi 做法）
    let _ = write!(stdout, "\x1b[?u");
    let _ = stdout.flush();
    if poll(Duration::from_millis(50)).unwrap_or(false) {
        let _ = read(); // 消费响应
        let _ = execute!(
            stdout,
            PushKeyboardEnhancementFlags(
                KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                    | KeyboardEnhancementFlags::REPORT_ALTERNATE_KEYS
            )
        );
    }
    ratatui::init()
}

fn main() -> Result<(), Box<dyn Error>> {
    PathResolver::global_init();

    let writer_path = pathes::path("log.txt");
    // Overwrite previous run logs at startup.
    OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&writer_path)?;
    let terminal = init_term();
    provider::REPEAT_DETECTION.store(true, Ordering::Relaxed);
    let mut app = App::new(terminal.into());
    tracing_subscriber::fmt()
        .with_timer(ChinaLocalTime)
        .with_writer(move || {
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(&writer_path)
                .expect("open log file failed")
        })
        .init();

    let (game_looper, reciver) = EventLooper::new(8, FRAME_DURATION.into());
    EventSender::init(game_looper.sender.clone());
    EventManager::init(game_looper);
    let res = App::main_loop(
        &mut app,
        &reciver,
        Duration::from_millis(FRAME_DURATION.into())
    );
    EventManager::with_looper(|l| l.stop());
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        AUDIOM.with(|a| { let _ = a.replace(None); });
    }));
    //  recorve origin terminal content, close mouse report
    execute!(
        app.terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;

    if let Err(err) = res {
        info!("{err:?}");
    }
    ratatui::restore();
    tracing::info!("process exit");
    std::process::exit(0);
}
