use chrono::prelude::*;
use crossterm::execute;
use crossterm::style::Color;
use crossterm::style::{
    Colors, Print, ResetColor, SetColors,
};
use log::{debug, error, info, trace, warn};
use log::{Level, LevelFilter, Metadata, Record, SetLoggerError};
use std::sync::atomic::{AtomicI32, AtomicI64, Ordering::Relaxed};

#[derive(Default)]
pub struct Logarithmic {
    last_second: AtomicI64,
    cursor_pos: AtomicI32,
}

impl Logarithmic {
    fn get_file_name(&self, record: &Record) -> String {
        let file_name = match record.file()
            .map(|f| std::path::Path::new(f))
            .map(|p| p.file_name())
            .flatten()
            .map(|s| s.to_str())
            .flatten()
            .map(|s| s.to_owned()) 
        {
            Some(s) => s,
            None => return String::new()
        };
        let line_number = match record.line() {
            Some(l) => l,
            None => return String::new()
        };
        format!("{}:{}", file_name, line_number)
    }

    fn get_time(&self) -> i64 {
        Utc::now().timestamp()
    }

    fn update_time(&self) {
        self.last_second.store(self.get_time(), Relaxed);
    }

    fn write_level(&self, level: Level) {
        let (foreground, background) = match level {
            Level::Warn => (Color::Red, None),
            Level::Info => (Color::DarkBlue, None),
            Level::Error => (Color::DarkRed, None),
            Level::Debug => (Color::Green, None),
            Level::Trace => (Color::Yellow, None),
        };

        self.write_string(
            &level.to_string(),
            Some(Colors {
                foreground: Some(foreground),
                background,
            }),
        );

        self.pad_to_column(17);
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
        } else {
            if let Err(_) = execute!(std::io::stdout(), ResetColor, Print(&format!("{}", text)),) {
                print!("{text}");
            }
        };
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
        self.write_string(&formatted_time.format("[%H:%M:%S] ").to_string(), None);
    }

    fn pad_to_column(&self, column_size: i32) {
        let mut column = String::new();
        for _ in 0..(column_size - self.cursor_pos.load(Relaxed)) {
            column += " ";
        }
        self.write_string(&column, None);
    }
}

impl log::Log for Logarithmic {
    fn enabled(&self, metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let width = crossterm::terminal::size().map(|ws| ws.0).unwrap_or(80);

            // TODO: Make struct level
            let padding_to_level = 17;
            self.write_time();
            self.write_level(record.level());
            let file_name = self.get_file_name(record);
            let lines = record
                .args()
                .to_string()
                .chars()
                .collect::<Vec<_>>()
                .chunks(width as usize - padding_to_level as usize - file_name.len() - 1)
                .map(|chunk| chunk.iter().collect::<String>())
                .collect::<Vec<String>>();

            let mut first_line = true;

            for line in lines {
                self.pad_to_column(padding_to_level);
                self.write_string(&line, None);
                if first_line {
                    // TODO (eatmynerds): change filename color to be gray
                    self.pad_to_column((width as usize - file_name.len()) as i32);
                    self.write_string(&file_name, None);
                    first_line = false;
                }

                self.add_newline();
            }
        }
    }

    fn flush(&self) {}
}

pub fn init() -> Result<(), SetLoggerError> {
    log::set_boxed_logger(Box::new(Logarithmic::default()))
        .map(|()| log::set_max_level(LevelFilter::Trace))
}

fn main() {
    init().unwrap();
    debug!("hi");
    info!("hi");
    warn!("hi");
    error!("hi");
    trace!("hi");
}
