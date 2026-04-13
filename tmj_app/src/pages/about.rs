
use ratatui::{crossterm::event::KeyCode, layout::Rect, widgets::Paragraph};
use tmj_core::{command::{CmdBuffer, GameCmd}, event::handler::EventDispatcher};
use crate::pages::{Screen, UserScreen};


pub struct AboutScene{ 
    text: String,
}
impl Screen for AboutScene {}

impl AboutScene {
    pub fn spawn(name_args: std::collections::HashMap<&str, &str>) -> Self {
        AboutScene{
            text: "dev: frostar".to_string()
        }
    }
}

impl super::Draw for AboutScene{
    fn draw(&self, frame: &mut ratatui::Frame, area: Rect) {
        let p = Paragraph::new(self.text.clone());
        frame.render_widget(p, area);
    }
}

impl EventDispatcher for AboutScene {
    fn on_key(&mut self, key: &ratatui::crossterm::event::KeyEvent){
        if key.is_release(){
            return;
        }
        match key.code {
            KeyCode::Esc=> {
                CmdBuffer::push(GameCmd::GoScene(UserScreen::Main.to_string()));
            }
            _ => {}
        }
    }

    fn on_quit(&mut self) {

    }

    fn on_mouse(&mut self, mouse: &ratatui::crossterm::event::MouseEvent) {

    }

    fn on_resize(&mut self, w: u16, h: u16)
    {

    }

    fn handle_tick(&mut self, tick: std::time::Duration) {}
}

