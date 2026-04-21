use std::{fs, sync::LazyLock};

use anyhow::Context;
use ratatui::layout::Constraint;
use serde::{Deserialize, Serialize};
use tmj_core::pathes;

#[derive(Serialize, Deserialize, Clone)]
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
    pub chapter_title_ltwh: (u16, u16, u16, u16),
    pub chapter_subtitle_ltwh: (u16, u16, u16, u16),
}

impl Layout {
    pub fn ltwh2rect(
        area: ratatui::layout::Rect,
        ltwh: &(u16, u16, u16, u16),
    ) -> ratatui::layout::Rect {
        let res_rect =
            ratatui::layout::Layout::vertical([Constraint::Length(ltwh.3), Constraint::Fill(1)])
                .split(area)[0];
        let face_rect =
            ratatui::layout::Layout::horizontal([Constraint::Length(ltwh.2), Constraint::Fill(1)])
                .split(res_rect)[0];

        let face_rect =
            face_rect.offset(ratatui::layout::Offset::new(ltwh.0 as i32, ltwh.1 as i32));
        face_rect.intersection(area)
    }
}

fn read_layout_file() -> anyhow::Result<Layout> {
    let path = pathes::path("layout.toml");
    if fs::exists(&path)? {
        let cnt = fs::read_to_string(path).context("current layout file unreadable!")?;
        Ok(toml::from_str::<Layout>(&cnt)?)
    } else {
        let layout = Layout::default();
        let cnt = toml::to_string(&layout)?;
        fs::write(path, cnt)?;
        Ok(layout)
    }
}

pub static LAYOUT: LazyLock<Layout> = LazyLock::new(|| match read_layout_file() {
    Ok(layout) => layout,
    Err(e) => {
        tracing::error!("when try load or create layout file: {:?}", e);
        Layout::default()
    }
});

impl Default for Layout {
    fn default() -> Self {
        Self {
            character_twh: (6, 80, 56),
            vertical_dark_edge: 5,
            two_character_spec: 20,
            x_character_spec: 16,
            frame_face_ltwh: (20, 47, 41, 22),
            frame_name_ltwh: (60, 49, 10, 1),
            frame_content_ltwh: (60, 50, 144, 16),
            text_ltwh: (61, 51, 140, 15),
            short_key_ltwh: (60, 66, 144, 1),
            chapter_title_ltwh: (60, 27, 120, 5),
            chapter_subtitle_ltwh: (60, 32, 120, 1),
        }
    }
}
