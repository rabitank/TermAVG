use std::{fs, ops::Div, path::PathBuf, sync::LazyLock};

use anyhow::{Context, Result};
use ratatui::layout::Constraint;
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
    pub layout: Layout,
}

#[derive(Serialize, Deserialize)]
pub struct Layout {
    pub character_twh: (u16, u16, u16),
    pub frame_face_ltwh: (u16, u16, u16, u16),
    pub vertical_dark_edge: u16,
    pub two_character_spec: u16,
    pub x_character_spec: u16,
    pub frame_content_ltwh: (u16, u16, u16, u16),
    pub text_ltwh: (u16, u16, u16, u16),
    pub frame_name_ltwh: (u16, u16, u16, u16),
    pub short_key_ltwh: (u16, u16, u16, u16),
}

impl Layout {
    pub fn ltwh2rect(
        area: ratatui::layout::Rect,
        ltwh: &(u16, u16, u16, u16),
    ) -> ratatui::layout::Rect {
        let res_rect = ratatui::layout::Layout::vertical([
            Constraint::Length(ltwh.3 as u16),
            Constraint::Fill(1),
        ])
        .split(area)[0];
        let face_rect =
            ratatui::layout::Layout::horizontal([Constraint::Length(ltwh.2), Constraint::Fill(1)])
                .split(res_rect)[0];

        let face_rect =
            face_rect.offset(ratatui::layout::Offset::new(ltwh.0 as i32, ltwh.1 as i32));
        face_rect.intersection(area)
    }
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

impl Default for Layout {
    fn default() -> Self {
        Self {
            character_twh: (8, 80, 56), // 16, 56 ,128
            vertical_dark_edge: 8,      // 12
            two_character_spec: 48,
            x_character_spec: 16,
            frame_face_ltwh: (20, 60, 41, 22), // 16, 112, 48 48
            frame_name_ltwh: (60, 62, 10, 1),
            frame_content_ltwh: (60, 63, 144, 16),
            text_ltwh: (61, 64, 140, 15),
            short_key_ltwh: (60, 79, 144, 1),
        }
    }
}

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
            layout: Layout::default(),
        }
    }
}
