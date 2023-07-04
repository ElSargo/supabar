use std::time::Instant;

use ansi_term::{Color, Style};
use chrono::{DateTime, Local, Timelike};
use unicode_width::UnicodeWidthStr;
use zellij_tile::prelude::*;
use zellij_utils::data::Palette;

#[derive(Default)]
struct State {
    tabs: Vec<TabInfo>,
    session_name: String,
    colors: Palette,
    time: String,
    mode: String,
    clip_message_time: Option<Instant>,
    branch: Option<String>,
}

register_plugin!(State);

impl ZellijPlugin for State {
    fn load(&mut self) {
        zellij_tile::prelude::set_timeout(0.0);
        #[cfg(not(debug_assertions))]
        {
            zellij_tile::prelude::set_selectable(false);
        }
        self.branch = std::fs::read_to_string("/host/.git/HEAD")
            .ok()
            .map(|mut s| s.split_off(16))
            .map(|name| name.chars().take_while(|c| !c.is_whitespace()).collect())
            .map(|name: String| format!("  {name} "));
        subscribe(&[
            EventType::ModeUpdate,
            EventType::TabUpdate,
            EventType::FileSystemUpdate,
            EventType::Timer,
            EventType::PaneUpdate,
            EventType::CopyToClipboard,
        ]);
    }

    fn update(&mut self, event: Event) -> bool {
        let mut should_render = false;
        match event {
            Event::CopyToClipboard(_) => {
                self.clip_message_time = Some(std::time::Instant::now());
                should_render = true;
            }
            Event::ModeUpdate(mode_info) => {
                let mode = format!("{:?}", mode_info.mode);
                self.mode = mode;
                self.session_name = mode_info.session_name.unwrap_or_default();
                self.colors = mode_info.style.colors;
                should_render = true;
            }
            Event::TabUpdate(tab_info) => {
                self.tabs = tab_info.clone();
                should_render = true;
            }
            Event::Timer(_) => {
                // Clock
                self.time = time();

                //Clipboard
                self.clip_message_time.map(|time| {
                    if Instant::now().duration_since(time).as_secs() > 2 {
                        self.clip_message_time = None;
                    }
                });

                zellij_tile::prelude::set_timeout(1.0);
                should_render = true;
            }
            _ => (),
        };
        should_render
    }

    fn render(&mut self, _rows: usize, cols: usize) {
        let into_col = |color| match color {
            PaletteColor::Rgb((r, g, b)) => Color::RGB(r, g, b),
            PaletteColor::EightBit(b) => Color::Fixed(b),
        };
        let white = into_col(self.colors.white);
        let gray = into_col(self.colors.black);
        let orange = into_col(self.colors.orange);
        let purple = into_col(self.colors.red);
        let black = into_col(self.colors.fg);
        let blue = into_col(self.colors.blue);

        let session = color(white, gray, &format!(" {} ", self.session_name));
        let session_width = self.session_name.width() + 2;
        let mode = color(
            if self.mode == "Normal" {
                blue
            } else if self.mode == "Locked" {
                purple
            } else {
                orange
            },
            black,
            &format!("{}", self.mode.to_uppercase()),
        );
        let mode = format!(
            "{}{}{}",
            color(black, gray, ""),
            mode,
            color(black, gray, ""),
        );

        let mode_width = self.mode.width() + 2;
        let time = color(white, gray, &format!(" {} ", self.time));
        let time_width = 13;

        let clip_message = if self.clip_message_time.is_some() {
            "Coppied!"
        } else {
            ""
        };
        let clip_width = clip_message.width();
        let clip = color(white, gray, clip_message);

        let (tabs, tabs_width) = render_tabs(
            &self.tabs,
            into_col(self.colors.green),
            black,
            gray,
            into_col(self.colors.orange),
            white,
        );
        let (branch, branch_width) = match &self.branch {
            Some(name) => (
                format!(
                    "{}{}{}",
                    color(black, gray, ""),
                    color(blue, black, &name),
                    color(gray, black, "")
                ),
                name.width() + 2,
            ),

            None => ("".to_owned(), 0),
        };

        let left = [session, mode, tabs].join("");
        let right = [clip, branch, time].join("");
        let content_len: usize =
            session_width + mode_width + tabs_width + clip_width + time_width + branch_width;
        let filler = color(
            gray,
            gray,
            &vec![' '; cols.saturating_sub(content_len)]
                .iter()
                .collect::<String>(),
        );
        let output = [left, filler, right].join("");
        print!("{output}",);

        // println!("{session}");
    }
}

fn render_tabs(
    info: &[TabInfo],
    green: Color,
    black: Color,
    gray: Color,
    orange: Color,
    white: Color,
) -> (String, usize) {
    let mut res = String::new();
    let mut total_width = 0;
    // NORMAL   master  +21 ~22 -14  󰀪 4 󰌶 4 

    for tab in info {
        let mut extras = Vec::new();
        if tab.is_fullscreen_active {
            extras.push("F");
        }
        if tab.is_sync_panes_active {
            extras.push("S");
        }
        if tab.are_floating_panes_visible {
            extras.push("f");
        }
        let tab_width = 3
            + tab.name.width()
            + if extras.is_empty() {
                0
            } else {
                2 + 2 * extras.len()
            };

        total_width += tab_width;
        let f = extras.join(" ");
        let c = if extras.is_empty() {
            format!(" {}", tab.name)
        } else {
            let (l, r, n) = if tab.active {
                (
                    color(orange, green, "<"),
                    color(orange, green, ">"),
                    color(white, green, &f),
                )
            } else {
                (
                    color(orange, black, "<"),
                    color(orange, black, ">"),
                    color(white, black, &f),
                )
            };
            format!(" {} {l}{}{r}", tab.name, n)
        };

        let t = if tab.active {
            format!(
                "{}{}{}",
                color(gray, green, ""),
                color(black, green, &c),
                color(green, gray, ""),
            )
        } else {
            format!(
                "{}{}{}",
                color(gray, black, ""),
                color(gray, black, &c),
                color(black, gray, ""),
            )
        };
        res.push_str(&t);
    }
    (res, total_width)
}

pub const CYAN: u8 = 51;
pub const GRAY_LIGHT: u8 = 238;
pub const GRAY_DARK: u8 = 245;
pub const WHITE: u8 = 15;
pub const BLACK: u8 = 16;
pub const RED: u8 = 124;
pub const GREEN: u8 = 154;
pub const ORANGE: u8 = 166;

fn color(fg: Color, bg: Color, text: &str) -> String {
    format!("{}", Style::new().fg(fg).on(bg).bold().paint(text))
}

fn time() -> String {
    let local: DateTime<Local> = Local::now();
    let hour = local.hour();
    let minute = local.minute();
    let second = local.second();

    let hour_12 = if hour > 12 { hour - 12 } else { hour };
    let am_pm = if hour >= 12 { "PM" } else { "AM" };

    format!("{:02}:{:02}:{:02} {}", hour_12, minute, second, am_pm)
}
