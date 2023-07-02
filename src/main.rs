use ansi_term::{Colour::Fixed, Style};
use owo_colors::OwoColorize;
use zellij_tile::prelude::*;

#[derive(Default)]
struct State {
    tabs: Vec<TabInfo>,
    session_name: String,
    git_status: String,
}

register_plugin!(State);

impl ZellijPlugin for State {
    fn load(&mut self) {
        subscribe(&[EventType::ModeUpdate, EventType::TabUpdate, EventType::FileSystemUpdate]);
    }

    fn update(&mut self, event: Event) -> bool {
        let mut should_render = false;
        match event {
            Event::ModeUpdate(mode_info) => {
                let mode = format!("{:?}", mode_info.mode);
                self.session_name = mode_info.session_name.unwrap_or_default();
                should_render = true;
            }
            Event::TabUpdate(tab_info) => {
                self.tabs = tab_info.clone();
                should_render = true;
            }
            Event::FileSystemUpdate( path ) => {
               self.git_status = git_info::get().current_branch.unwrap_or_default();
                
            }
            _ => (),
        };
        should_render
    }

    fn render(&mut self, _rows: usize, cols: usize) {
        let session = format!(" {} ", self.session_name);
        let session = session.on_bright_black();
        let session = session.bold();

        let tabs = render_tabs(&self.tabs);

        let git = &self.git_status; 

        println!("{session}{tabs}{git}");

    }
}

fn render_tabs(info: &[TabInfo]) -> String {
    let mut res = String::new();

    // NORMAL   master  +21 ~22 -14  󰀪 4 󰌶 4 

    for tab in info {
        let t = if tab.active {
            let c = format!(" {}", tab.name)
                .on_bright_red()
                .black()
                .bold()
                .to_string();
            format!(
                "{}{}{}",
                "".bright_black().on_bright_red(),
                c,
                "".bright_red().on_bright_black(),
            )
        } else {
            let c = format!(" {}", tab.name)
                .on_black()
                .bright_black()
                .bold()
                .to_string();
            format!(
                "{}{}{}",
                "".bright_black().on_black(),
                c,
                "".black().on_bright_black(),
            )
        };
        res.push_str(&t);
    }
    res
}

pub const CYAN: u8 = 51;
pub const GRAY_LIGHT: u8 = 238;
pub const GRAY_DARK: u8 = 245;
pub const WHITE: u8 = 15;
pub const BLACK: u8 = 16;
pub const RED: u8 = 124;
pub const GREEN: u8 = 154;
pub const ORANGE: u8 = 166;

fn color_bold(color: u8, text: &str) -> String {
    format!("{}", Style::new().fg(Fixed(color)).bold().paint(text))
}
