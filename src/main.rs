use anyhow::anyhow;
use ratatui::crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen, enable_raw_mode};
use std::error::Error;
use std::time::Duration;
use tmj_app::app::App;
use tmj_core::event::EventManager;
use tmj_core::event::looper::EventLooper;
use tmj_core::event::sender::EventSender;
use tmj_core::pathes::PathResolver;
use tracing::info;

const FRAME_DURATION: u8 = (1000 / 60) as u8;

fn init_term() -> ratatui::Terminal<ratatui::prelude::CrosstermBackend<std::io::Stdout>> {
    let _ = enable_raw_mode();
    let mut stdout = std::io::stdout();
    // switch terminal buffer, enable mouse trace
    let _ = execute!(stdout, EnterAlternateScreen, EnableMouseCapture);
    ratatui::init()
}

fn main() -> Result<(), Box<dyn Error>> {
    PathResolver::global_init();

    let terminal = init_term();
    let mut app = App::new(terminal.into());
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr) // 关键：输出到 stderr！
        .init();

    let res = {
        if let Ok((game_looper, reciver)) = EventLooper::new(8, FRAME_DURATION.into()) {
            EventSender::init(game_looper.sender.clone());
            EventManager::init(game_looper);
            App::main_loop(
                &mut app,
                &reciver,
                Duration::from_millis(FRAME_DURATION.into()),
            )
        } else {
            Err(anyhow!("Create EventLooper failed!"))
        }
    };
    //  recorve origin terminal content, close mouse report
    execute!(
        app.terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;

    if let Err(err) = res {
        info!("{err:?}");
    }
    Ok(())
}
