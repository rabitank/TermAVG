use std::{fs, ops::Div, path::PathBuf, sync::LazyLock};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tmj_core::pathes;

use crate::setting;

#[derive(Serialize, Deserialize)]
pub struct GameSetting {
    pub resolution: (u16, u16),
    pub is_force_skipable: bool,
    pub save_dir: PathBuf,
    pub entre_script: PathBuf,
    pub default_bg_img: PathBuf,
    pub default_face_img: PathBuf,
    pub layout: Layout,
}

#[derive(Serialize, Deserialize)]
pub struct Layout {
    pub character_up_edge: usize,
    pub character_size: (u16, u16),
    pub df_size: (u16, u16), // dialouge face
    pub df_lt: (u16, u16),
    pub dark_edge: u16,
    pub two_character_spec: u16,
    pub x_character_spec: u16,
    pub dtf_lr_edge: u16, // dialouge text frame
    pub dtf_height: u16,
    pub text_lt: (u16, u16),
    pub text_size: (u16, u16),
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
            entre_script: "resource/script.fs".into(),
            default_bg_img: "resource/default_background_img.png".into(),
            default_face_img: "resource/default_face_img.png".into(),
            layout: Layout {
                character_up_edge: 24.div(2) as usize,
                character_size: (56, 112.div(2) as u16),
                dark_edge: 12 / 2,
                two_character_spec: 48,
                x_character_spec: 16,
                dtf_lr_edge: 32,
                dtf_height: 32 / 2,
                text_lt: (64, resolution.1 as u16 - 32 / 2),
                text_size: (136, 28 / 2),
                df_size: (40, 40 / 2),
                df_lt: (16, resolution.1 as u16 - 48 / 2),
            },
        }
    }
}
