use ansi_term::{Color, Style};
use chrono::{DateTime, Local, Timelike};
use std::time::Instant;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};
use zellij_tile::prelude::*;
// use zellij_utils::data::Palette;

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
        match event {
            Event::CopyToClipboard(_) => {
                self.clip_message_timer = Some(std::time::Instant::now());
            }
            Event::ModeUpdate(mode_info) => {
                let mode = format!("{:?}", mode_info.mode);
                self.mode = mode;
                self.session_name = mode_info.session_name.unwrap_or_default();
                self.colors = mode_info.style.colors;
            }
            Event::TabUpdate(tab_info) => {
                self.tabs = tab_info;
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
            }
            _ => (),
        };
        // All paths require a rerender
        true
    }

    fn render(&mut self, _rows: usize, cols: usize) {
        let white = into_color(self.colors.white);
        let gray = into_color(self.colors.black);
        let orange = into_color(self.colors.orange);
        let purple = into_color(self.colors.red);
        let black = into_color(self.colors.fg);
        let blue = into_color(self.colors.blue);
        let green = into_color(self.colors.green);

        let session = color_and_bold(white, gray, &format!(" {} ", self.session_name));
        let session_width = self.session_name.width() + 2;
        let mode = color_and_bold(
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
            color_and_bold(black, gray, ""),
            mode,
            color_and_bold(black, gray, ""),
        );
        let active_tab = self.tabs.iter().find(|tab| tab.active);
        let layout = active_tab.and_then(|tab| tab.active_swap_layout_name.as_ref());

        let (layout, layout_width) = match &layout {
            Some(name) => (
                if active_tab.is_some_and(|b| b.is_swap_layout_dirty) {
                    format!(
                        "{}{}{}",
                        color_and_bold(green, gray, ""),
                        color_and_bold(gray, green, &format!("{} ", name.to_uppercase())),
                        color_and_bold(gray, green, "")
                    )
                } else {
                    format!(
                        "{}{}{}",
                        color_and_bold(green, gray, ""),
                        color_and_bold(black, green, &format!("{} ", name.to_uppercase())),
                        color_and_bold(gray, green, "")
                    )
                },
                name.width() + 3,
            ),

            None => (String::new(), 0),
        };

        let mode_width = self.mode.width() + 2;
        let time = color_and_bold(white, gray, &format!(" {} ", self.time));
        let time_width = 13;

        let clip_message = if self.clip_message_timer.is_some() {
            "Coppied! "
        } else {
            ""
        };
        let clip_width = clip_message.width();
        let clip = color_and_bold(white, gray, clip_message);

        let (tabs, tabs_width) = render_tabs(&self.tabs, green, black, gray, orange, white);
        let (branch, branch_width) = match &self.branch {
            Some(name) => (
                format!(
                    "{}{}{}",
                    color_and_bold(black, gray, ""),
                    color_and_bold(blue, black, name),
                    color_and_bold(black, gray, "")
                ),
                name.width() + 2,
            ),

            None => (String::new(), 0),
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
        let filler = color_and_bold(
            gray,
            gray,
            &vec![' '; cols.saturating_sub(content_len)]
                .iter()
                .collect::<String>(),
        );
        let content = [left, filler, right].join("");

        let output = if content_len > cols {
            let plus = color_and_bold(white, orange, "+");
            let chrs = get_chars_to_truncate(&content, cols.saturating_sub(1));
            format!("{}{plus}", content.chars().take(chrs).collect::<String>())
        } else {
            content
        };

        print!("{output}");
    }
}

/// Trivial conversion betwwen zellij palettte and terminal color
fn into_color(color: PaletteColor) -> Color {
    match color {
        PaletteColor::Rgb((r, g, b)) => Color::RGB(r, g, b),
        PaletteColor::EightBit(b) => Color::Fixed(b),
    }
}

// Inlined from https://lib.rs/crates/textwrap textwrap::core::display_width
/// Returns the number of chars to keep in order to produce a string with width columns.
/// The columns is the displayed width in the terminal
/// Merley calling UnicodeWidthChar::width is insufficeint due to ansi escape sequneces
fn get_chars_to_truncate(output: &str, columns: usize) -> usize {
    /// ignored when computing the text width.
    const CSI: (char, char) = ('\x1b', '[');
    /// The final bytes of an ANSI escape sequence must be in this range.
    const ANSI_FINAL_BYTE: std::ops::RangeInclusive<char> = '\x40'..='\x7e';
    let mut computed_characters = 0;
    {
        let mut chars = output.chars();
        let mut computed_columns = 0;
        while let Some(char) = chars.next() {
            computed_characters += 1;
            let mut skip_ansi = || {
                let chars = &mut chars;
                if char == CSI.0 && chars.next() == Some(CSI.1) {
                    computed_characters += 1;
                    // We have found the start of an ANSI escape code, typically
                    // used for colored terminal text. We skip until we find a
                    // "final byte" in the range 0x40–0x7E.
                    for ch in chars {
                        computed_characters += 1;
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
            computed_columns += char.width().unwrap_or(0);
            if computed_columns == columns {
                break;
            }
        }
        computed_columns
    };
    computed_characters
}

/// Renders the tabs section, returns the content and the width
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
                    color_and_bold(orange, green, "<"),
                    color_and_bold(orange, green, ">"),
                    color_and_bold(white, green, &f),
                )
            } else {
                (
                    color_and_bold(orange, black, "<"),
                    color_and_bold(orange, black, ">"),
                    color_and_bold(white, black, &f),
                )
            };
            format!(" {} {l}{}{r}", tab.name, n)
        };

        let t = if tab.active {
            format!(
                "{}{}{}",
                color_and_bold(gray, green, ""),
                color_and_bold(black, green, &c),
                color_and_bold(green, gray, ""),
            )
        } else {
            format!(
                "{}{}{}",
                color_and_bold(gray, black, ""),
                color_and_bold(gray, black, &c),
                color_and_bold(black, gray, ""),
            )
        };
        res.push_str(&t);
    }
    (res, total_width)
}

/// Apply a foreground and background color, and make the text bold
fn color_and_bold(fg: Color, bg: Color, text: &str) -> String {
    format!("{}", Style::new().fg(fg).on(bg).bold().paint(text))
}

/// Formatted local time
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

    format!("{hour_12:02}:{minute:02}:{second:02} {am_pm}")
}
