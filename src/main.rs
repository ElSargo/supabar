use ansi_term::{Color, Style};
use chrono::{DateTime, Local, Timelike};
use std::time::Instant;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};
use zellij_tile::prelude::*;

const LSEP: &str = "";
const RSEP: &str = "";

#[derive(Default)]
struct State {
    tabs: Vec<TabInfo>,
    session_name: String,
    time: String,
    mode: String,
    clip_message_timer: Option<Instant>,
    branch: Option<String>,
    colors: Colors,
}

impl Default for Colors {
    fn default() -> Self {
        Self {
            white: Color::Black,
            gray: Color::Black,
            orange: Color::Black,
            purple: Color::Black,
            black: Color::Black,
            blue: Color::Black,
            green: Color::Black,
        }
    }
}

struct Colors {
    black: Color,
    blue: Color,
    gray: Color,
    green: Color,
    orange: Color,
    purple: Color,
    white: Color,
}

register_plugin!(State);

impl ZellijPlugin for State {
    fn load(&mut self) {
        #[cfg(not(debug_assertions))]
        zellij_tile::prelude::set_selectable(false);

        zellij_tile::prelude::set_timeout(0.0);
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
                self.clip_message_timer = Some(Instant::now());
            }
            Event::ModeUpdate(mode_info) => {
                let mode = format!("{:?}", mode_info.mode);
                self.mode = mode;
                self.session_name = mode_info.session_name.unwrap_or_default();
                self.colors = Colors {
                    black: into_color(mode_info.style.colors.fg),
                    blue: into_color(mode_info.style.colors.blue),
                    gray: into_color(mode_info.style.colors.black),
                    green: into_color(mode_info.style.colors.green),
                    orange: into_color(mode_info.style.colors.orange),
                    purple: into_color(mode_info.style.colors.red),
                    white: into_color(mode_info.style.colors.white),
                }
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
        let State {
            tabs,
            session_name,
            time,
            mode,
            clip_message_timer,
            branch,
            colors,
        } = self;
        let session = color_and_bold(colors.white, colors.gray, &format!(" {} ", session_name));
        let session_width = session_name.width() + 2;
        let mode_color = if mode == "Normal" {
            colors.blue
        } else if mode == "Locked" {
            colors.purple
        } else {
            colors.orange
        };

        let mode_width = mode.width() + 2;
        let mode_content = color_concat(
            (colors.black, colors.gray, LSEP),
            (mode_color, colors.black, &mode.to_uppercase()),
            (colors.black, colors.gray, RSEP),
        );

        let active_tab = tabs.iter().find(|tab| tab.active);
        let layout = active_tab.and_then(|tab| tab.active_swap_layout_name.as_ref());

        let format_layout = |accent, name: &str| {
            color_concat(
                (colors.green, colors.gray, LSEP),
                (accent, colors.green, &format!("{} ", name.to_uppercase())),
                (colors.gray, colors.green, LSEP),
            )
        };
        let (layout, layout_width) = match &layout {
            Some(name) => (
                if active_tab.is_some_and(|b| b.is_swap_layout_dirty) {
                    format_layout(colors.green, name)
                } else {
                    format_layout(colors.black, name)
                },
                name.width() + 3,
            ),

            None => (String::new(), 0),
        };

        let time = color_and_bold(colors.white, colors.gray, &format!(" {} ", time));
        let time_width = 13;

        let clip_message = if clip_message_timer.is_some() {
            "Coppied! "
        } else {
            ""
        };
        let clip_width = clip_message.width();
        let clip = color_and_bold(colors.white, colors.gray, clip_message);

        let (tabs, tabs_width) = render_tabs(&tabs, &colors);
        let (branch, branch_width) = match &branch {
            Some(name) => (
                color_concat(
                    (colors.black, colors.gray, LSEP),
                    (colors.blue, colors.black, name),
                    (colors.black, colors.gray, RSEP),
                ),
                name.width() + 2,
            ),

            None => (String::new(), 0),
        };

        let left = [session, mode_content, tabs].join("");
        let right = [clip, layout, branch, time].join("");
        let content_len: usize = session_width
            + mode_width
            + tabs_width
            + clip_width
            + time_width
            + branch_width
            + layout_width;
        let filler = color_and_bold(
            colors.gray,
            colors.gray,
            &vec![' '; cols.saturating_sub(content_len)]
                .iter()
                .collect::<String>(),
        );
        let content = [left, filler, right].join("");

        let output = if content_len > cols {
            let plus = color_and_bold(colors.white, colors.orange, "+");
            let chrs = get_chars_to_truncate(&content, cols.saturating_sub(1));
            let truncated = content.chars().take(chrs).collect::<String>();
            [truncated, plus].concat()
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
fn render_tabs(info: &[TabInfo], colors: &Colors) -> (String, usize) {
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
            let fmt = |col| {
                color_concat(
                    (colors.orange, col, "<"),
                    (colors.white, col, &f),
                    (colors.orange, col, ">"),
                )
            };
            let tab_content = if tab.active {
                fmt(colors.green)
            } else {
                fmt(colors.black)
            };
            format!(" {} {tab_content }", tab.name)
        };

        let t = if tab.active {
            color_concat(
                (colors.gray, colors.green, RSEP),
                (colors.gray, colors.green, &c),
                (colors.green, colors.gray, RSEP),
            )
        } else {
            color_concat(
                (colors.gray, colors.black, RSEP),
                (colors.gray, colors.black, &c),
                (colors.black, colors.gray, RSEP),
            )
        };
        res.push_str(&t);
    }
    (res, total_width)
}

/// Apply a foreground and background color, and make the text bold
fn color_and_bold(fg: Color, bg: Color, text: &str) -> String {
    Style::new().fg(fg).on(bg).bold().paint(text).to_string()
}

fn color_concat(
    c1: (Color, Color, &str),
    c2: (Color, Color, &str),
    c3: (Color, Color, &str),
) -> String {
    [
        color_and_bold(c1.0, c1.1, c1.2),
        color_and_bold(c2.0, c2.1, c2.2),
        color_and_bold(c3.0, c3.1, c3.2),
    ]
    .concat()
}

/// Formatted local time
fn time() -> String {
    let local: DateTime<Local> = Local::now();
    let hour = local.hour();
    let minute = local.minute();
    let second = local.second();

    let hour_12 = if hour == 0 {
        12
    } else if hour > 12 {
        hour - 12
    } else {
        hour
    };
    let am_pm = if hour >= 12 { "AM" } else { "PM" };

    format!("{hour_12:02}:{minute:02}:{second:02} {am_pm}")
}
