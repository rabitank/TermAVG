
use ratatui::style::{Color, Modifier, Style};

pub struct Theme {
    pub root: Style,
    pub content: Style,
    pub app_title: Style,
    pub tabs: Style,
    pub tabs_selected: Style,
    pub borders: Style,
    pub description: Style,
    pub description_title: Style,
    pub key_binding: KeyBinding,
    pub logo: Logo,
    pub dialouge: Dialogue,
    pub load: Load,
    pub save: Save,
    pub history: History
}

pub struct KeyBinding {
    pub key: Style,
    pub description: Style,
}

pub struct Logo {
    pub rat_eye: Color,
    pub rat_eye_alt: Color,
}

pub struct Dialogue {
    pub tabs: Style,
    pub tabs_selected: Style,
    pub inbox: Style,
    pub item: Style,
    pub selected_item: Style,
    pub header: Style,
    pub header_value: Style,
    pub block: Style,
    pub black_edge: Style,
    pub name: Style,
    pub charpter_subtitle: Style,
    pub charpter_title: Style,
    pub background: Style,
}

pub struct Load {
    pub header: Style,
    pub selected: Style,
    pub ping: Style,
    pub map: Map,
}

pub struct Map {
    pub style: Style,
    pub color: Color,
    pub path: Color,
    pub source: Color,
    pub destination: Color,
    pub background_color: Color,
}

pub struct Save {
    pub ingredients: Style,
    pub ingredients_header: Style,
}

pub struct History {
    pub base: Style,
    pub item_border: Style,
    pub say_item: Style,
    pub text_item: Style
}

pub const THEME: Theme = Theme {
    root: Style::new().bg(DARK_BLUE),
    content: Style::new().bg(DARK_BLUE).fg(LIGHT_GRAY),
    app_title: Style::new()
        .fg(WHITE)
        .bg(DARK_BLUE)
        .add_modifier(Modifier::BOLD),
    tabs: Style::new().fg(MID_GRAY).bg(DARK_BLUE),
    tabs_selected: Style::new()
        .fg(WHITE)
        .bg(DARK_BLUE)
        .add_modifier(Modifier::BOLD)
        .add_modifier(Modifier::REVERSED),
    borders: Style::new().fg(LIGHT_GRAY),
    description: Style::new().fg(LIGHT_GRAY).bg(DARK_BLUE),
    description_title: Style::new().fg(LIGHT_GRAY).add_modifier(Modifier::BOLD),
    logo: Logo {
        rat_eye: BLACK,
        rat_eye_alt: RED,
    },
    key_binding: KeyBinding {
        key: Style::new().fg(BLACK).bg(MID_GRAY),
        description: Style::new().fg(MID_GRAY).bg(BLACK),
    },
    dialouge: Dialogue {
        name: Style::new().fg(WHITE).bg(BLACK),
        tabs: Style::new().fg(MID_GRAY).bg(DARK_BLUE),
        tabs_selected: Style::new()
            .fg(WHITE)
            .bg(DARK_BLUE)
            .add_modifier(Modifier::BOLD),
        inbox: Style::new().bg(DARK_BLUE).fg(LIGHT_GRAY),
        item: Style::new().fg(LIGHT_GRAY),
        selected_item: Style::new().fg(LIGHT_YELLOW),
        header: Style::new().add_modifier(Modifier::BOLD),
        header_value: Style::new().fg(LIGHT_GRAY),
        block: Style::new().bg(DARK_BLUE).fg(LIGHT_GRAY),
        black_edge: Style::new().bg(BLACK).fg(WHITE),
        charpter_subtitle: Style::new().fg(LIGHT_GRAY),
        charpter_title: Style::new().fg(WHITE),
        background: Style::new().bg(BLACK),
    },
    load: Load {
        header: Style::new()
            .bg(DARK_BLUE)
            .add_modifier(Modifier::BOLD)
            .add_modifier(Modifier::UNDERLINED),
        selected: Style::new().fg(LIGHT_YELLOW),
        ping: Style::new().fg(WHITE),
        map: Map {
            style: Style::new().bg(DARK_BLUE),
            background_color: DARK_BLUE,
            color: LIGHT_GRAY,
            path: LIGHT_BLUE,
            source: LIGHT_GREEN,
            destination: LIGHT_RED,
        },
    },
    save: Save {
        ingredients: Style::new().bg(DARK_BLUE).fg(LIGHT_GRAY),
        ingredients_header: Style::new()
            .add_modifier(Modifier::BOLD)
            .add_modifier(Modifier::UNDERLINED),
    },
    history: History{
        base: Style::new().bg(DARK_BLUE),
        item_border: Style::new().fg(LIGHT_GRAY),
        say_item: Style::new().fg(WHITE),
        text_item: Style::new().fg(LIGHT_GRAY)
    }
};

pub const DARK_BLUE: Color = Color::Rgb(16, 24, 48);
pub const LIGHT_BLUE: Color = Color::Rgb(64, 96, 192);
pub const LIGHT_YELLOW: Color = Color::Rgb(192, 192, 96);
pub const LIGHT_GREEN: Color = Color::Rgb(64, 192, 96);
pub const LIGHT_RED: Color = Color::Rgb(192, 96, 96);
pub const RED: Color = Color::Rgb(215, 0, 0);
pub const BLACK: Color = Color::Rgb(8, 8, 8); // not really black, often #080808
pub const DARK_GRAY: Color = Color::Rgb(68, 68, 68);
pub const MID_GRAY: Color = Color::Rgb(128, 128, 128);
pub const LIGHT_GRAY: Color = Color::Rgb(188, 188, 188);
pub const WHITE: Color = Color::Rgb(238, 238, 238); // not really white, often #eeeeee
pub const LTY_BLUE: Color = Color::from_u32(0x66ccff_u32);
