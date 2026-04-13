use std::cell::RefCell;
use std::rc::Rc;
use strum_macros::Display;

#[derive(Clone)]
pub struct CmdBuffer {
    // Rc<RefCell<>> 允许在逻辑层任意位置 push，但仅限单线程（ratatui 适用）
    inner: Rc<RefCell<Vec<GameCmd>>>,
}

thread_local! {
    static COMMANDBUFFER: RefCell<Vec<GameCmd>> = RefCell::new(Vec::with_capacity(32));
}
impl CmdBuffer {
    /// 任意位置可调用（无需 &mut self）
    pub fn push(cmd: GameCmd) {
        COMMANDBUFFER.with_borrow_mut(|v| v.push(cmd));
    }

    /// 仅 AppState 在帧结束时调用（消费并清空）
    pub fn take_commands() -> Vec<GameCmd> {
        COMMANDBUFFER.with(|b| b.borrow_mut().drain(..).collect::<Vec<_>>())
    }
}

#[derive(Display, Debug)]
pub enum SaveSlot {
    Temp,
    #[strum(to_string = "Slots {0}")]
    Slots(u8)
}

#[derive(Display, Debug)]
pub enum GameCmd {
    #[strum(to_string = "GoScene {0}")]
    GoScene(String),

    #[strum(to_string = "GoBack")]
    GoBack,
    
    #[strum(to_string = "EntreGame {0}")]
    EntreGame(String),

    #[strum(to_string = "SaveTo {0}")]
    SaveTo(SaveSlot),

    #[strum(to_string = "LoadFrom {0}")]
    LoadFrom(SaveSlot),

    #[strum(to_string = "Popup {0} at {1}")]
    Popup(String, ratatui::layout::Rect),

    NewGame,
    QuitGame,
    ContinueGame,
}

pub trait CmdDispatcher {
    //  cmd 是游戏内命令， GameCmd是由字符串解析而来的一个字符串数组数据。
    //  cmd是一层层处理的，GameFlow相关只会在App的handle cmd中处理， 因此会返回true表示终止传播。
    //  如果不认识的主语对象,那么基本上需要继续传播下去
    fn handle_cmd(&mut self, _cmd: &GameCmd) -> anyhow::Result<bool> {
       Ok(false) 
    }
}
