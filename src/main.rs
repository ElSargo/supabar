use ansi_term::{Color, Style};
use chrono::{DateTime, Local, Timelike};
use std::time::Instant;
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
    clip_message_timer: Option<Instant>,
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
            .map(|name: String| format!(" {name}"));
        subscribe(&[
            EventType::ModeUpdate,
            EventType::TabUpdate,
            EventType::FileSystemUpdate,
            EventType::Timer,
            EventType::CopyToClipboard,
        ]);
    }

    fn update(&mut self, event: Event) -> bool {
        let mut should_render = false;
        match event {
            Event::CopyToClipboard(_) => {
                self.clip_message_timer = Some(std::time::Instant::now());
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
                self.tabs = tab_info;
                should_render = true;
            }
            Event::Timer(_) => {
                // Clock
                self.time = time();

                //Clipboard
                if let Some(time) = self.clip_message_timer {
                    if Instant::now().duration_since(time).as_secs() > 2 {
                        self.clip_message_timer = None;
                    }
                }

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
        let green = into_col(self.colors.green);

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
            &self.mode.to_uppercase(),
        );
        let mode = format!(
            "{}{}{}",
            color(black, gray, ""),
            mode,
            color(black, gray, ""),
        );
        let active_tab = self.tabs.iter().find(|tab| tab.active);
        let layout = active_tab.and_then(|tab| tab.active_swap_layout_name.as_ref());

        let (layout, layout_width) = match &layout {
            Some(name) => (
                if active_tab.is_some_and(|b| b.is_swap_layout_dirty) {
                    format!(
                        "{}{}{}",
                        color(green, gray, ""),
                        color(gray, green, &format!("{} ", name.to_uppercase())),
                        color(gray, green, "")
                    )
                } else {
                    format!(
                        "{}{}{}",
                        color(green, gray, ""),
                        color(black, green, &format!("{} ", name.to_uppercase())),
                        color(gray, green, "")
                    )
                },
                name.width() + 3,
            ),

            None => ("".to_owned(), 0),
        };

        let mode_width = self.mode.width() + 2;
        let time = color(white, gray, &format!(" {} ", self.time));
        let time_width = 13;

        let clip_message = if self.clip_message_timer.is_some() {
            "Coppied! "
        } else {
            ""
        };
        let clip_width = clip_message.width();
        let clip = color(white, gray, clip_message);

        let (tabs, tabs_width) = render_tabs(&self.tabs, green, black, gray, orange, white);
        let (branch, branch_width) = match &self.branch {
            Some(name) => (
                format!(
                    "{}{}{}",
                    color(black, gray, ""),
                    color(blue, black, name),
                    color(black, gray, "")
                ),
                name.width() + 2,
            ),

            None => ("".to_owned(), 0),
        };

        let left = [session, mode, tabs].join("");
        let right = [clip, layout, branch, time].join("");
        let content_len: usize = session_width
            + mode_width
            + tabs_width
            + clip_width
            + time_width
            + branch_width
            + layout_width;
        let filler = color(
            gray,
            gray,
            &vec![' '; cols.saturating_sub(content_len)]
                .iter()
                .collect::<String>(),
        );
        let content = [left, filler, right].join("");

        let output = if content_len > cols {
            let plus = color(white, orange, "+");
            let chrs = compute_truncated_lenght(&content, cols.saturating_sub(1));
            format!("{}{plus}", content.chars().take(chrs).collect::<String>())
        } else {
            content
        };

        // let actual = textwrap::core::display_width(&output);
        // println!("computed {} columns {} actual {}", cos, cols, actual);

        print!("{output}");

        // println!("{session}");
    }
}

// Inlined from https://lib.rs/crates/textwidth
fn compute_truncated_lenght(output: &str, cols: usize) -> usize {
    /// ignored when computing the text width.
    const CSI: (char, char) = ('\x1b', '[');
    /// The final bytes of an ANSI escape sequence must be in this range.
    const ANSI_FINAL_BYTE: std::ops::RangeInclusive<char> = '\x40'..='\x7e';
    let mut chrs = 0;
    {
        let text: &str = output;
        let mut chars = text.chars();
        let mut width = 0;
        while let Some(ch) = chars.next() {
            chrs += 1;
            let mut skip_ansi = || {
                let chars = &mut chars;
                if ch == CSI.0 && chars.next() == Some(CSI.1) {
                    chrs += 1;
                    // We have found the start of an ANSI escape code, typically
                    // used for colored terminal text. We skip until we find a
                    // "final byte" in the range 0x40–0x7E.
                    for ch in chars {
                        chrs += 1;
                        if ANSI_FINAL_BYTE.contains(&ch) {
                            return true;
                        }
                    }
                }
                false
            };
            if skip_ansi() {
                continue;
            }
            width += unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
            if width == cols {
                break;
            }
        }
        width
    };
    chrs
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

fn color(fg: Color, bg: Color, text: &str) -> String {
    format!("{}", Style::new().fg(fg).on(bg).bold().paint(text))
}

fn time() -> String {
    let local: DateTime<Local> = Local::now();
    let hour = local.hour();
    let minute = local.minute();
    let second = local.second();

    let hour_12 = if hour == 12 || hour == 0 {
        12
    } else if hour > 12 {
        hour - 12
    } else {
        hour
    };
    let am_pm = if hour >= 12 { "AM" } else { "PM" };

    format!("{:02}:{:02}:{:02} {}", hour_12, minute, second, am_pm)
}
