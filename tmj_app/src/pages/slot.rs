use std::{
    cell::RefCell,
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    rc::Rc,
};

use anyhow::Ok;
use ratatui::{
    Frame,
    crossterm::event::KeyCode,
    layout::Rect,
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{List, ListItem, ListState},
};
use regex::Regex;
use tmj_core::{
    event::handler::EventDispatcher,
};

use crate::{SETTING, art::theme::{self}};

pub const SLOT_SIZE: usize = 20;

#[derive(Debug)]
pub struct Slot {
    pub path: Option<PathBuf>,
    pub time: time::OffsetDateTime,
    pub name: String,
    pub id: u8,
}

impl Slot {
    pub fn ensure_slot_path(&mut self) -> anyhow::Result<PathBuf> {
        if self.path.is_none() {
            let file_name = format!("{}_{}.save", self.id, self.name);
            let mut path = SETTING.abs_save_dir()?;
            path.push(file_name);
            self.path = Some(path);
        }
        Ok(self.path.clone().unwrap())
    }
    pub fn slot_pattern() -> &'static str {
        r"^\d+_.*.save$"
    }
}

pub struct SlotManager {
    slot_selections: HashMap<u8, Slot>,
    list_state: RefCell<ListState>,
}

impl EventDispatcher for SlotManager {
    fn on_key(&mut self, key: &ratatui::crossterm::event::KeyEvent) {
        if key.is_release() {
            return;
        }
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                self.list_state.borrow_mut().select_next();
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.list_state.borrow_mut().select_previous();
            }
            KeyCode::Char('g') => {
                self.list_state.borrow_mut().select_first();
            }
            KeyCode::Char('G') => {
                self.list_state.borrow_mut().select_last();
            }
            _ => {}
        }
    }
}

impl SlotManager {
    pub fn new() -> anyhow::Result<Self> {
        let abs_dir = SETTING.abs_save_dir()?;
        let slot_map = SlotManager::find_save_files(&abs_dir)?;
        let mut list_state = ListState::default();
        list_state.select_first();
        Ok(Self {
            slot_selections: slot_map,
            list_state: list_state.into(),
        })
    }

    pub fn check_any_save_slot(&self) -> bool {
        for s in self.slot_selections.values() {
            if s.path.is_some() {
                return true;
            }
        }
        return false;
    }


    pub fn get_current_slot(&mut self) -> Option<&mut Slot> {
        let pos = self.list_state.borrow_mut().selected();
        match pos {
            Some(p) => self.get_slot(p).ok(),
            None => None,
        }
    }

    pub fn get_slot(&mut self, slot_id: usize) -> anyhow::Result<&mut Slot> {
        let slot_id: u8 = slot_id as u8;
        match self.slot_selections.get_mut(&slot_id) {
            Some(_slot) => Ok(_slot),
            None => anyhow::bail!("wrong slot id".to_string()),
        }
    }

    /// 获取指定目录下一层中符合模式的文件路径列表
    fn find_save_files(dir: &Path) -> anyhow::Result<HashMap<u8, Slot>> {
        // 编译正则表达式（在函数外定义可避免重复编译）
        let re = Regex::new(Slot::slot_pattern()).expect("无效的正则表达式");
        let mut matches: HashMap<u8, Slot> = fs::read_dir(dir)?
            .filter_map(|entry| {
                let entry = match entry.ok() {
                    Some(e) => e,
                    None => return None,
                };
                let path = entry.path();
                // 只处理文件，忽略目录
                if path.is_file() {
                    if let Some(filename) = path.file_name().and_then(|s| s.to_str()) {
                        if re.is_match(filename) {
                            let file_prefix = path.file_prefix().unwrap().to_str().unwrap().to_string();
                            let splits: Vec<String> =
                                file_prefix.split('_').map(|s| s.to_string()).collect();
                            let id: u8 = splits[0].parse().unwrap();
                            let name = splits[1].clone();
                            let meta_data = std::fs::metadata(&path).ok().unwrap();
                            let modify_time: time::OffsetDateTime =
                                meta_data.modified().unwrap().into();

                            return Some(Slot {
                                path: Some(path),
                                time: modify_time,
                                name,
                                id,
                            });
                        }
                    }
                }
                None
            })
            .map(|s| (s.id, s))
            .collect();

        let now = if let Result::Ok(_now) = time::OffsetDateTime::now_local() {
            _now
        } else {
            time::OffsetDateTime::now_utc()
        };
        for slot_id in 0..SLOT_SIZE {
            let slot_id = slot_id as u8;
            let slot = matches.get(&slot_id);
            if slot.is_none() {
                matches.insert(
                    slot_id as u8,
                    Slot {
                        name: "".into(),
                        path: None,
                        id: slot_id as u8,
                        time: now.clone(),
                    },
                );
            }
        }
        Ok(matches)
    }
}

impl super::Draw for SlotManager {
    fn draw(&self, frame: &mut Frame, area: Rect) {
        let mut menu_items: Vec<ListItem> = Vec::with_capacity(SLOT_SIZE);

        for pos in 0..SLOT_SIZE {
            let pos = pos as u8;
            let slot = self.slot_selections.get(&pos);
            if slot.is_none() {
                tracing::error!("{} Slot Get Failed when render slotlist", pos);
                break;
            }
            let slot = slot.unwrap();
            let _widget = match slot.path {
                Some(_) => {
                    let text = Line::from_iter([
                        Span::from(format!("Slot {:^2} ", slot.id)).bold().fg(theme::LTY_BLUE),
                        Span::from(format!(
                            "{:<18} {}",
                            slot.name,
                            slot.time.truncate_to_second()
                        ))
                        .style(theme::LTY_BLUE),
                    ]);
                    text
                }
                None => {
                    let text = Line::from_iter([
                        Span::from(format!("Slot {:^2} ", pos)).bold(),
                        Span::from(format!("{:<18}", "Empty")).style(Color::Gray),
                    ]);
                    text
                }
            };
            menu_items.push(_widget.into());
        }

        let menu_ls = List::new(menu_items)
            .highlight_symbol(">>")
            .highlight_style(Style::default().bg(Color::White).fg(Color::Black));

        frame.render_stateful_widget(menu_ls, area, &mut *self.list_state.borrow_mut());
    }
}

thread_local! {
pub static SAVE_MANAGER: Rc<RefCell<SlotManager>> =
     Rc::new(RefCell::new(SlotManager::new().unwrap()));

}
