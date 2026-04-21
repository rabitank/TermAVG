use std::{fs, path::PathBuf, sync::LazyLock};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tmj_core::pathes;

#[derive(Serialize, Deserialize)]
pub struct GameSetting {
    pub resolution: (u16, u16),
    pub preprogress_script: Vec<String>, // 需要预处理的脚本路径
    pub is_force_skipable: bool,
    pub save_dir: PathBuf,
    pub entre_script: PathBuf,
    pub default_bg_img: PathBuf,
    pub default_face_img: PathBuf,
}

impl GameSetting {
    pub fn abs_save_dir(&self) -> Result<PathBuf, std::io::Error> {
        let slot_dir = self.save_dir.clone();
        let slot_dir = pathes::path(slot_dir);
        pathes::ensure_dir(slot_dir)
    }

    pub fn entre_script_path(&self) -> Result<PathBuf, std::io::Error> {
        let script_path = self.entre_script.clone();
        let script_path = pathes::path(script_path);
        pathes::ensure_file(script_path)
    }
}

fn read_setting_file() -> anyhow::Result<GameSetting> {
    let path = pathes::path("setting.toml");
    let setting = if fs::exists(&path)? {
        let cnt = fs::read_to_string(path).context("current setting file unreadable!")?;
        let game_setting = toml::from_str::<GameSetting>(&cnt)?;
        game_setting
    } else {
        let game_setting = GameSetting::default();
        let cnt = toml::to_string(&game_setting)?;
        fs::write(path, cnt)?;
        game_setting
    };
    Ok(setting)
}

pub static SETTING: LazyLock<GameSetting> = LazyLock::new(|| {
    // try build cfg dir and read from file/or create default cfg file
    match read_setting_file() {
        Ok(setting) => setting,
        Err(e) => {
            tracing::error!("when try load or create setting file: {:?}", e);
            GameSetting::default()
        }
    }
});

impl Default for GameSetting {
    fn default() -> Self {
        let resolution = (240, 160 / 2);
        Self {
            resolution: resolution, // 3: 2, 但是注意这里的尺寸也是按照字符宽高比为1:2来计算的
            is_force_skipable: false,
            save_dir: "save".into(),
            preprogress_script: Vec::new(),
            entre_script: "resource/script.fs".into(),
            default_bg_img: "resource/default_background_img.png".into(),
            default_face_img: "resource/default_face_img.png".into(),
        }
    }
}
