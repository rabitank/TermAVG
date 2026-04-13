use anyhow::Context;
use anyhow::Result;
use std::cell::RefCell;
use std::sync::mpsc::Receiver;
use std::thread;
use std::time::Duration;
use tmj_core::command::CmdBuffer;
use tmj_core::event::EventManager;

use ratatui::DefaultTerminal;

use tmj_core::event::{GameEvent, handler::EventDispatcher};

use crate::game::Game;

pub struct App {
    pub terminal: DefaultTerminal,
    pub game: RefCell<Game>,
}

impl App {
    pub fn new(terminal: DefaultTerminal) -> App {
        let game: RefCell<Game> = Game::new().into();
        App { terminal, game }
    }

    pub fn main_loop(
        app: &mut App,
        receiver: &Receiver<GameEvent>,
        tick_rate: Duration,
    ) -> Result<()> {
        let mut last_tick = std::time::Instant::now();
        let mut game = app.game.borrow_mut();
        EventManager::with_looper(|l| {
            l.cool_down(Duration::from_millis(100));
        });
        loop {
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
            app.terminal.draw(|f| game.draw(f));

            if last_tick.elapsed() < tick_rate {
                thread::sleep(tick_rate - last_tick.elapsed());
            }

            if game.game_flow.borrow().is_ready_quit() {
                break Ok(());
            }
        }
    }
}
