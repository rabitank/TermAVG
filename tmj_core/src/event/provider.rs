use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};

use super::GameEvent;

pub static REPEAT_DETECTION: AtomicBool = AtomicBool::new(false);

pub trait EventProvider: Send {
    fn poll_event(&mut self) -> Option<GameEvent>;
}

pub struct CrosstermProvider {
    poll_timeout: Duration,
    last_key: Option<(KeyCode, Instant)>,
}

impl CrosstermProvider {
    pub fn new(poll_timeout: Duration) -> Self {
        Self { poll_timeout, last_key: None }
    }
}

impl EventProvider for CrosstermProvider {
    fn poll_event(&mut self) -> Option<GameEvent> {
        if event::poll(self.poll_timeout).ok()? {
            let ct = event::read().ok()?;
            let mut ge = convert_crossterm_event(ct);
            if REPEAT_DETECTION.load(Ordering::Relaxed) {
                if let GameEvent::CtKeyEvent(ref mut key) = ge {
                    let now = Instant::now();
                    if let Some((last_code, last_at)) = self.last_key {
                        if last_code == key.code && now.duration_since(last_at) < Duration::from_millis(50) {
                            key.kind = KeyEventKind::Repeat;
                        }
                    }
                    self.last_key = Some((key.code, now));
                }
            }
            Some(ge)
        } else {
            None
        }
    }
}

pub struct NoopProvider;

impl EventProvider for NoopProvider {
    fn poll_event(&mut self) -> Option<GameEvent> {
        None
    }
}

pub fn convert_crossterm_event(ct_event: Event) -> GameEvent {
    match &ct_event {
        Event::Key(key) => {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        return GameEvent::QuitGame;
                    }
                    _ => return GameEvent::CtKeyEvent(*key),
                }
            } else {
                return GameEvent::CtKeyEvent(*key);
            }
        }
        Event::Resize(w, h) => GameEvent::ResizeTerm(*w, *h),
        Event::Mouse(mouse) => GameEvent::CtMouseEvent(*mouse),
        _ => GameEvent::CtUnDefined,
    }
}
