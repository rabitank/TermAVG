use std::{
    cell::RefCell,
    fs::File,
    io::BufReader,
};

use anyhow::Result;
use strum_macros::Display;
use tmj_core::{
    audio::{AudioManager, AudioSource},
    pathes, script::lower_str,
};

#[derive(Clone, Hash, PartialEq, Debug, Display)]
pub enum Tracks {
    Bgm,
    Voice,
    Effect,
    Effect1,
    Effect2,
}

impl Eq for Tracks {}

pub fn load_audio(file: impl ToString) -> Result<AudioSource> {
    let path = pathes::path(file.to_string());
    let file = File::open(path)?;
    let source = rodio::Decoder::new(BufReader::new(file))?;
    Ok(Box::new(source))
}

thread_local! {
    pub static AUDIOM: RefCell<AudioManager<Tracks>> = RefCell::new(AudioManager::new().unwrap());
}


lower_str!(FADE_IN);
lower_str!(FADE_OUT);
lower_str!(TRANSITION);
