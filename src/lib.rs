use chrono::prelude::*;
use crossterm::{
    execute,
    style::{Color, Colors, Print, ResetColor, SetColors},
};
use log::{Level, LevelFilter, Record, SetLoggerError};
use std::sync::atomic::Ordering::Relaxed;
#[cfg(feature = "async")]
#[path = "async.rs"]
mod log_mode;
#[cfg(not(feature = "async"))]
#[path = "sync.rs"]
pub(crate) mod log_mode;

use log_mode::{RichLogger, LOGGER};

#[cfg(feature = "json")]
pub(crate) mod json;
#[cfg(feature = "json")]
use json::{print_json_color, JsonToken};

pub(crate) struct RichLoggerRecord {
    pub(crate) file_name: String,
    pub(crate) level: Level,
    pub(crate) content: ContentType,
}

pub(crate) enum ContentType {
    TextContent(String),
    #[cfg(feature = "json")]
    JsonContent(Vec<JsonToken>),
}

fn file_name(record: &Record) -> String {
    let file_name = match record
        .file()
        .map(|f| std::path::Path::new(f))
        .map(|p| p.file_name())
        .flatten()
        .map(|s| s.to_str())
        .flatten()
        .map(|s| s.to_owned())
    {
        Some(s) => s,
        None => return String::new(),
    };
    let line_number = match record.line() {
        Some(l) => l,
        None => return String::new(),
    };
    format!("{}:{}", file_name, line_number)
}

impl<'l> From<Record<'l>> for RichLoggerRecord {
    fn from(value: Record<'l>) -> Self {
        RichLoggerRecord {
            file_name: file_name(&value),
            level: value.level(),
            content: ContentType::TextContent(value.args().to_string()),
        }
    }
}

enum TabStop {
    Time,
    Level,
    Content,
}

impl RichLogger {
    fn get_time(&self) -> i64 {
        Utc::now().timestamp()
    }

    fn update_time(&self) {
        self.last_second.store(self.get_time(), Relaxed);
    }

    fn tab_stop(&self, tab_stop: TabStop) -> i32 {
        match tab_stop {
            TabStop::Time => 0,
            TabStop::Level => 11,
            TabStop::Content => 17,
        }
    }

    fn write_level(&self, level: Level) {
        let (foreground, background) = match level {
            Level::Warn => (Color::Yellow, None),
            Level::Info => (Color::White, None),
            Level::Error => (Color::Black, Some(Color::Red)),
            Level::Debug => (Color::Cyan, None),
            Level::Trace => (Color::Green, None),
        };

        self.write_string(
            &level.to_string(),
            Some(Colors {
                foreground: Some(foreground),
                background,
            }),
        );
    }

    fn write_string(&self, text: &str, colors: Option<Colors>) {
        self.cursor_pos.fetch_add(text.len() as i32, Relaxed);

        if let Some(colors) = colors {
            if let Err(_) = execute!(
                std::io::stdout(),
                SetColors(colors),
                Print(&format!("{}", text)),
            ) {
                print!("{text}");
            }
            if let Err(_) = execute!(std::io::stdout(), ResetColor, Print("")) {}
        } else {
            if let Err(_) = execute!(std::io::stdout(), ResetColor, Print(&format!("{}", text)),) {
                print!("{text}");
            }
        }
    }

    fn add_newline(&self) {
        println!("");
        self.cursor_pos.store(0, Relaxed);
    }

    fn write_time(&self) {
        if self.last_second.load(Relaxed) == self.get_time() {
            return self.pad_to_column(11);
        }
        self.update_time();
        let formatted_time = match DateTime::from_timestamp(self.last_second.load(Relaxed), 0) {
            Some(s) => s.with_timezone(&Local),
            None => {
                return self.pad_to_column(11);
            }
        };
        self.write_string(
            &formatted_time.format("[%H:%M:%S] ").to_string(),
            Some(Colors {
                foreground: Some(Color::Grey),
                background: None,
            }),
        );
    }

    fn pad_to_column(&self, column_size: i32) {
        let mut column = String::new();
        for _ in 0..(column_size - self.cursor_pos.load(Relaxed)) {
            column += " ";
        }
        self.write_string(&column, None);
    }
}

pub(crate) fn log_impl(record: RichLoggerRecord) {
    let logger = &*LOGGER;
    let width = crossterm::terminal::size().map(|ws| ws.0).unwrap_or(80);
    logger.pad_to_column(logger.tab_stop(TabStop::Time));
    logger.write_time();
    logger.pad_to_column(logger.tab_stop(TabStop::Level));
    logger.write_level(record.level);

    match &record.content {
        ContentType::TextContent(t) => {
            let lines = t
                .replace("\t", "    ")
                .replace("\r", "")
                .chars()
                .collect::<Vec<_>>()
                .chunks(
                    width as usize
                        - logger.tab_stop(TabStop::Content) as usize
                        - record.file_name.len()
                        - 1,
                )
                .map(|chunk| chunk.iter().collect::<String>())
                .collect::<Vec<String>>();

            let mut first_line = true;

            for line in lines {
                logger.pad_to_column(logger.tab_stop(TabStop::Content));
                logger.write_string(&line, None);

                if first_line {
                    logger.pad_to_column((width as usize - record.file_name.len()) as i32);
                    logger.write_string(
                        &record.file_name,
                        Some(Colors {
                            foreground: Some(Color::Grey),
                            background: None,
                        }),
                    );
                    first_line = false;
                }

                execute!(std::io::stdout(), ResetColor).ok();
                logger.add_newline();
            }
        }
        #[cfg(feature = "json")]
        ContentType::JsonContent(j) => {
            logger.pad_to_column(logger.tab_stop(TabStop::Content));
            print_json_color(&record, &j);
            logger.add_newline();
        }
    }
}

pub fn init(level: LevelFilter) -> Result<(), SetLoggerError> {
    log::set_logger(&*LOGGER).map(|()| log::set_max_level(level))
}
