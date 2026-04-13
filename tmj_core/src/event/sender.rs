use std::
    sync::{
        Mutex, OnceLock, mpsc::SyncSender
    }
;

use anyhow::{Context, Result};

use crate::event::GameEvent;

static INSTANCE: OnceLock<Mutex<EventSender>> = OnceLock::new();

pub struct EventSender {
    sender: SyncSender<GameEvent>,
}

impl EventSender {
    pub fn init(sender: SyncSender<GameEvent>) {
        INSTANCE.set(Mutex::new(EventSender { sender }));
    }

    pub fn with_sender<F>(f: F)
    where
        F: FnOnce(&mut SyncSender<GameEvent>),
    {

        INSTANCE.get().map(|mutex| {
            let mut guard = mutex.lock().unwrap();
            f(&mut guard.sender);
        });
    }

    pub fn sender_event(event: GameEvent) -> Result<()>{
       INSTANCE.get().unwrap().lock().unwrap().sender.send(event).context("Sender send event failed!")
    }
}
