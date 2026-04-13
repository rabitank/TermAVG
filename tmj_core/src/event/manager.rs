use crate::event::looper::EventLooper;
use std::{
    sync::{Mutex, OnceLock},
    time::Duration,
};

static INSTANCE: OnceLock<Mutex<EventManager>> = OnceLock::new();

pub struct EventManager {
    looper: EventLooper,
}

impl EventManager {
    pub fn init(looper: EventLooper) {
        let _ = INSTANCE.set(Mutex::new(EventManager { looper }));
    }

    pub fn with_looper<F>(f: F)
    where
        F: FnOnce(&mut EventLooper),
    {
        INSTANCE.get().map(|mutex| {
            let mut guard = mutex.lock().unwrap();
            f(&mut guard.looper);
        });
    }
    pub fn cool_down(duration: Duration) {
        EventManager::with_looper(|l| {
            l.cool_down(duration);
        });
    }
}
