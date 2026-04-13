use std::{
    sync::{
        Arc,
        atomic::AtomicBool,
        mpsc::{self, Receiver, SyncSender},
    }, thread, time::{Duration, Instant}
};

use ratatui::crossterm::
    event::{self, KeyEventKind}
;
use ratatui::crossterm::
    event::{Event, KeyCode, KeyModifiers}
;
use crate::event::events::GameEvent;
use anyhow::{Context, Result};

pub struct EventLooper {
    pub sender: SyncSender<GameEvent>,
    close_flag: Arc<AtomicBool>,
    _thread_handle: Option<thread::JoinHandle<()>>,
    /// 启动时间戳，用于实现"启动保护期"
    start_time: Instant,
    /// 启动保护期时长（毫秒），此期间内忽略某些事件
    warmup_duration: Duration,
}


impl EventLooper {
    pub fn new(buffer_size: usize, poll_timeout: u64) -> Result<(Self, Receiver<GameEvent>)> {
        let (sender, reciver) = mpsc::sync_channel(buffer_size);
        let close_flag = Arc::new(AtomicBool::new(false));
        let close_flag_cloned = close_flag.clone();
        let sender_cloned = sender.clone();

        let thread_handle = thread::spawn(move || {
            if let Err(e) = Self::event_collection_loop(sender_cloned, close_flag_cloned, poll_timeout) {
                eprintln!("Evenet collection error: {}", e);
            }
        });

        Ok((
            Self {
                sender,
                close_flag,
                _thread_handle: Some(thread_handle),
                start_time: Instant::now(),
                warmup_duration: Duration::from_millis(500)
            },
            reciver,
        ))
    }

    fn event_collection_loop(
        sender: SyncSender<GameEvent>,
        close_flag: Arc<AtomicBool>,
        poll_timeout: u64,
    ) -> Result<()> {
        let timeout = Duration::from_millis(poll_timeout);

        loop {
            if close_flag.load(std::sync::atomic::Ordering::SeqCst) {
                break;
            }

            if event::poll(timeout).context("EventLoop poll crossterm event field")? {
                let ct_event = event::read()?;
                let game_event = Self::convert_event(ct_event);
                if matches!(&game_event, GameEvent::QuitGame) {
                    let _ = sender.send(game_event);
                    break;
                }
                let _ = sender.send(game_event);
            }
        }
        Ok(())
    }
    /// 清空事件缓冲区（在游戏主循环开始前调用）
    pub fn drain_buffer(&self, receiver: &Receiver<GameEvent>) {
        // 消耗掉所有残留事件
        while receiver.try_recv().is_ok() {
            // 丢弃所有事件
        }
    }

    /// 检查是否仍在启动保护期内
    pub fn check_is_warmup(&self) -> bool {
        self.start_time.elapsed() > self.warmup_duration
    }

    pub fn cool_down(&mut self, duration: Duration) {
        self.start_time = Instant::now();
        self.warmup_duration = duration;
    }

    fn convert_event(ct_event: Event) -> GameEvent {
        match &ct_event {
            Event::Key(key) => {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) =>
                        {
                            return GameEvent::QuitGame;
                        }
                        _ => return GameEvent::CtKeyEvent(*key),
                    }
                } else {
                    return GameEvent::CtKeyEvent(*key);
                }
            }
            Event::Resize(w, h) => return GameEvent::ResizeTerm(*w, *h),
            Event::Mouse(mouse) => return GameEvent::CtMouseEvent(*mouse),
            _ => return GameEvent::CtUnDefined,
        }
    }

    pub fn stop(&self) {
        self.close_flag.store(true, std::sync::atomic::Ordering::SeqCst);
    }

}
