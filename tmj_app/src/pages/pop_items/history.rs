use std::cell::RefCell;

use ratatui::{
    crossterm::event::KeyCode,
    layout::{Alignment, Constraint, Layout, Margin},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, ListState, Padding, Paragraph},
};
use tmj_core::event::handler::EventDispatcher;

use crate::{
    art::theme,
    pages::{
        pipeline::{
            logical_area,
            visual_element::{VisualElement, VisualElementKind},
        },
        pop_items::{DialogueRecord, HISTORY_LS, PopItem},
    },
};

pub struct DialogueHistoryLs {
    list_state: RefCell<ListState>,
    dark_ve: VisualElement,
    shown: bool,
    scroll_offset: usize,
    ls_wh: RefCell<(u16, u16)>,
}

impl DialogueHistoryLs {
    pub fn new() -> Self {
        DialogueHistoryLs {
            list_state: RefCell::new(ListState::default()),
            shown: false,
            dark_ve: VisualElement {
                name: "_".into(),
                alpha: 0.4,
                style: Style::new().bg(crate::art::theme::BLACK),
                rect: logical_area(),
                fill_before_draw: true,
                kind: VisualElementKind::Text { content: "".into() },
                ..Default::default()
            },
            scroll_offset: 0,
            ls_wh: crate::layout::LAYOUT.history_wh.clone().into(),
        }
    }

    pub fn build_item_widget(&self, record: &DialogueRecord) -> Paragraph {
        // 为当前记录绘制一个带边框的段落
        let block = Block::bordered().border_style(theme::THEME.history.item_border).padding(Padding::new(2, 2, 1, 1));
        let block = if !record.speaker.is_empty() {
            block.title(format!(" {}", record.speaker)) // 边框标题显示说话人
        } else {
            block
        };

        let content = Paragraph::new("  ".to_owned() + &record.content.clone())
            .block(block)
            .wrap(ratatui::widgets::Wrap { trim: false });
        let content = if record.speaker.is_empty() {
            content.style(theme::THEME.history.text_item)
        } else {
            content.style(theme::THEME.history.say_item)
        };
        content
    }

    fn draw_history(&self, frame: &mut ratatui::Frame, area: ratatui::layout::Rect) {
        let history = super::HISTORY_LS.lock().unwrap();
        let records = history.records();
        if records.is_empty() {
            return;
        }
        let total = records.len();
        // 底部最后一条记录的索引
        // 底部索引（最新记录索引 total-1，减去 offset）
        let bottom_idx = total.saturating_sub(1 + self.scroll_offset);

        let bottom_idx = bottom_idx.min(total.saturating_sub(1));
        // 如果偏移过大导致底部索引为0且仍不够，直接显示第一条
        let max_y = area.bottom(); // 可视区域底部行号 (不包含快捷键栏)
        let mut current_y = max_y;

        // 跳过前面不可见的记录
        for idx in (0..=bottom_idx).rev() {
            let record = &records[idx];
            // 测量该 Paragraph 在给定宽度下的高度
            let content = self.build_item_widget(record);
            let item_h = content.line_count(area.width.max(1) as u16) as u16;
            let top_y = current_y.saturating_sub(item_h);
            // 如果上方已经超出可视区域顶部，停止绘制
            if top_y >= current_y {
                // 数值溢出保护
                break;
            }
            // 构造绘制区域，y 坐标可能为负也没关系，Widget 会自动处理
            let item_area = ratatui::layout::Rect::new(area.x, top_y, area.width, item_h);
            frame.render_widget(content, item_area);

            current_y = top_y; // 下一条记录从这之上开始画
            if current_y <= area.y {
                // 部分或完全超出顶部，但我们仍然画，Widget 会自动裁剪到 area 内
                break;
            }
        }
        self.set_ls_wh(area);
    }

    fn ls_height(&self) -> u16 {
        self.ls_wh.borrow().1
    }

    fn ls_width(&self) -> u16 {
        self.ls_wh.borrow().0
    }

    fn set_ls_wh(&self, area: ratatui::layout::Rect) {
        *self.ls_wh.borrow_mut() = (area.width, area.height);
    }
}

fn draw_shortkey_bar(frame: &mut ratatui::Frame, area: ratatui::layout::Rect) {
    let key_style = theme::THEME.key_binding.key;
    let desc_style = theme::THEME.key_binding.description;

    let line = Line::from(vec![
        Span::styled(" ↑/k ", key_style),
        Span::styled("上移 ", desc_style),
        Span::styled(" ↓/j ", key_style),
        Span::styled("下移 ", desc_style),
        Span::styled(" PgUp ", key_style),
        Span::styled("上翻页 ", desc_style),
        Span::styled(" PgDn ", key_style),
        Span::styled("下翻页 ", desc_style),
        Span::styled(" Home ", key_style),
        Span::styled("开头 ", desc_style),
        Span::styled(" End ", key_style),
        Span::styled("末尾 ", desc_style),
        Span::styled(" q ", key_style),
        Span::styled("退出", desc_style),
    ])
    .centered();
    // let paragraph = Paragraph::new(line)
    //     .alignment(Alignment::Center)
    //     .block(Block::new().borders(Borders::TOP)); // 顶部分界线
    frame.render_widget(line, area);
}

impl PopItem for DialogueHistoryLs {
    fn set_visual(&mut self, visual: bool) {
        self.scroll_offset = 0;
        self.shown = visual;
    }

    fn draw(&self, frame: &mut ratatui::Frame, rect: ratatui::layout::Rect) -> anyhow::Result<()> {
        if !self.shown {
            return Ok(());
        }
        self.dark_ve.render(frame.buffer_mut(), rect);

        let rect = rect.centered(
            Constraint::Length(crate::LAYOUT.history_wh.0),
            Constraint::Length(crate::LAYOUT.history_wh.1),
        );

        let [list_rect, short_key_rect] = rect.layout(&Layout::vertical([
            Constraint::Fill(1),
            Constraint::Length(1),
        ]));

        frame.render_widget(Clear, rect);
        frame.render_widget(
            Block::default().style(crate::art::theme::THEME.content),
            rect,
        );

        self.draw_history(frame, list_rect.inner(Margin::new(14, 4)));
        draw_shortkey_bar(frame, short_key_rect);

        Ok(())
    }

    fn is_show(&self) -> bool {
        self.shown
    }
}

impl EventDispatcher for DialogueHistoryLs {
    fn on_key(&mut self, key: &ratatui::crossterm::event::KeyEvent) {
        if self.is_hide() {
            return;
        }
        
        let total = super::HISTORY_LS.lock().unwrap().len();
        if total == 0 {
            *self.list_state.borrow_mut().offset_mut() = 0;
            return;
        }
        let binding = HISTORY_LS.lock().unwrap();
        let records = binding.records();
        match key.code {
            // 上/ k ：查看更早的消息 → 偏移增大
            KeyCode::Up | KeyCode::Char('k') if key.is_press() => {
                if self.scroll_offset < total.saturating_sub(1) {
                    self.scroll_offset += 1;
                }
            }
            // 下/ j ：查看更新的消息 → 偏移减小
            KeyCode::Down | KeyCode::Char('j') if key.is_press() => {
                self.scroll_offset = self.scroll_offset.saturating_sub(1);
            }
            // PageUp：向上翻页（更早）
            KeyCode::PageUp if key.is_release() => {
                let mut used_height = 0u16;
                let max_offset = total.saturating_sub(1);
                let mut new_offset = self.scroll_offset;
                // 从当前底部偏移开始，逐条累加高度，直到超出或不可再增加
                for offset in self.scroll_offset..max_offset {
                    let idx = total - 1 - offset; // 对应的记录索引
                    let h = self
                        .build_item_widget(&records[idx])
                        .line_count(self.ls_width()) as u16;
                    if used_height + h > self.ls_height() {
                        break;
                    }
                    used_height += h;
                    new_offset = offset + 1; // 下一条作为底部
                }
                self.scroll_offset = new_offset.min(max_offset);
            }
            // PageDown：向下翻页（更新）
            KeyCode::PageDown if key.is_release() => {
                if self.scroll_offset == 0 {
                    return;
                }
                let mut used_height = 0u16;
                let mut new_offset = self.scroll_offset;
                // 从当前偏移向前（更新的记录）累加，注意偏移减小的方向
                while new_offset > 0 {
                    let idx = total - 1 - (new_offset - 1); // 比当前底部更新的一条
                    if idx >= total {
                        break;
                    }
                    let h = self
                        .build_item_widget(&records[idx])
                        .line_count(self.ls_width()) as u16;
                    if used_height + h > self.ls_height() {
                        break;
                    }
                    used_height += h;
                    new_offset -= 1;
                }
                self.scroll_offset = new_offset;
            }
            // Home：跳到最旧的消息（偏移最大）
            KeyCode::Home if key.is_release() => {
                self.scroll_offset = total.saturating_sub(1);
            }
            // End：跳到最新的消息（偏移为 0）
            KeyCode::End if key.is_release() => {
                self.scroll_offset = 0;
            }
            KeyCode::Esc | KeyCode::Char('q') if key.is_release() => {
                self.hide();
            }
            _ => {}
        }
    }
}
