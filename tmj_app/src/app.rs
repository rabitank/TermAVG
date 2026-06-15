use anyhow::Context;
use ratatui::Terminal;
use ratatui::prelude::Backend;
use ratatui::widgets::Paragraph;
use ratatui::layout::Alignment;
use std::cell::RefCell;
use std::sync::mpsc::Receiver;
use std::thread;
use std::time::Duration;
use tmj_core::command::CmdBuffer;
use tmj_core::event::EventManager;

use tmj_core::event::{GameEvent, handler::EventDispatcher};

use crate::game::Game;
use crate::setting::SETTING;

pub struct App<T: Backend> {
    pub terminal: Terminal<T>,
    pub game: RefCell<Game>,
}

impl<T: Backend> App<T> {
    pub fn new(terminal: Terminal<T>) -> Self
    where
        T: Backend,
    {
        let game: RefCell<Game> = Game::new().into();
        App { terminal, game }
    }

    pub fn main_loop(
        app: &mut App<T>,
        receiver: &Receiver<GameEvent>,
        tick_rate: Duration,
    ) -> anyhow::Result<()>
    where
        T::Error: std::error::Error + Send + Sync + 'static,
    {
        let mut last_tick = std::time::Instant::now();
        let mut game = app.game.borrow_mut();
        EventManager::with_looper(|l| {
            l.cool_down(Duration::from_millis(100));
        });
        'main: loop {
            // 检测终端尺寸
            let req = SETTING.resolution;
            let size = app.terminal.backend().size()?;
            if size.width < req.0 || size.height < req.1 {
                app.terminal.draw(|f| {
                    let msg = format!(
                        "终端尺寸不足：需要 {}×{}，当前 {}×{}\n请调整终端字号(Ctrl+-调整)或放大窗口\n\n按 Ctrl+C 退出",
                        req.0, req.1, size.width, size.height
                    );
                    f.render_widget(
                        Paragraph::new(msg).alignment(Alignment::Center),
                        f.area(),
                    );
                }).map_err(|e| anyhow::anyhow!("{e:?}"))?;
                while let Ok(event) = receiver.try_recv() {
                    if matches!(event, GameEvent::QuitGame) {
                        break 'main Ok(());
                    }
                }
                last_tick = std::time::Instant::now();
                thread::sleep(tick_rate);
                continue;
            }

            let last_tick_time = last_tick.elapsed();
            last_tick = std::time::Instant::now();

            // 事件冷静, 即屏蔽事件接收一段时间
            EventManager::with_looper(|l| {
                if !l.check_is_warmup() {
                    l.drain_buffer(receiver);
                }
            });

            if let Ok(event) = receiver.try_recv() {
                if !game
                    .handle_event(&event)
                    .context("app handle event failed!")?
                {
                    return Ok(());
                }
            }

            game.handle_tick(last_tick_time);

            for cmd in CmdBuffer::take_commands() {
                game.handle_cmd(&cmd)
                    .context(format!("game handle cmd:{} failed!", cmd))?;
            }
            app.terminal.draw(|f| game.draw(f)).map_err(|e| anyhow::anyhow!("{e:?}"))?;

            thread::sleep(tick_rate.saturating_sub(last_tick.elapsed()));

            if game.game_flow.borrow().is_ready_quit() {
                break Ok(());
            }
        }
    }
}
