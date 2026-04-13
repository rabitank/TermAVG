
use ratatui::crossterm::event::{KeyEvent, MouseEvent};

#[derive(Debug, Clone, PartialEq)]
pub enum GameEvent {
    /// 原始 Crossterm 事件
    CtKeyEvent(KeyEvent),
    CtMouseEvent(MouseEvent),
    CtUnDefined,
    /// resize window
    ResizeTerm(u16, u16),
    /// 自定义游戏事件：退出游戏
    QuitGame,
}
