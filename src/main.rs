use chrono::prelude::*;
use crossterm::execute;
use crossterm::style::Color;
use crossterm::style::{
    Color::{Black, Green},
    Colors, Print, ResetColor, SetColors,
};
use log::{error, info};
use log::{Level, LevelFilter, Metadata, Record, SetLoggerError};
use std::io::Write;
use std::sync::atomic::{AtomicI32, AtomicI64, Ordering::Relaxed};

#[derive(Default)]
pub struct Logarithmic {
    last_second: AtomicI64,
    cursor_pos: AtomicI32,
}

impl Logarithmic {
    fn get_time(&self) -> i64 {
        let local_time: DateTime<Local> = Local::now();

        local_time.timestamp()
    }

    fn update_time(&self) {
        self.last_second.store(self.get_time(), Relaxed);
    }

    fn write_level(&self, level: Level) {
        match level {
            Level::Warn => {
                self.write_string(
                    &level.to_string(),
                    Some(Colors {
                        foreground: Some(Color::Red),
                        background: None,
                    }),
                );
            }
            Level::Info => {
                self.write_string(
                    &level.to_string(),
                    Some(Colors {
                        foreground: Some(Color::DarkBlue),
                        background: None,
                    }),
                );
            }
            Level::Error => {
                self.write_string(
                    &level.to_string(),
                    Some(Colors {
                        foreground: Some(Color::DarkRed),
                        background: None,
                    }),
                );
            }
            Level::Debug => {
                self.write_string(
                    &level.to_string(),
                    Some(Colors {
                        foreground: Some(Color::Green),
                        background: None,
                    }),
                );
            }
            Level::Trace => {
                self.write_string(
                    &level.to_string(),
                    Some(Colors {
                        foreground: Some(Color::Red),
                        background: None,
                    }),
                );
            }
        }

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
        if self.last_second.load(Relaxed) != self.get_time() {
            self.update_time();
            if let Some(formatted_time) =
                DateTime::from_timestamp(self.last_second.load(Relaxed), 0)
            {
                let formatted_time = formatted_time.format("[%H:%M:%S] ").to_string();
                self.write_string(&formatted_time, None)
            } else {
                self.pad_to_column(11);
            }
        } else {
            self.pad_to_column(11);
        }
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
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let width = crossterm::terminal::size()
                .map(|ws| ws.0)
                .unwrap_or(80);
            // TODO: Make struct level
            let padding_to_level = 20;
            self.write_time();
            self.write_level(record.level());
            let lines = record
                .args()
                .to_string()
                .chars()
                .collect::<Vec<_>>()
                .chunks(width as usize - padding_to_level as usize)
                .map(|chunk| chunk.iter().collect::<String>())
                .collect::<Vec<String>>();
            for line in lines {
                self.pad_to_column(padding_to_level);
                self.write_string(&line, None);
                self.add_newline();
            }
        }
    }

    fn flush(&self) {}
}

pub fn init() -> Result<(), SetLoggerError> {
    log::set_boxed_logger(Box::new(Logarithmic::default()))
        .map(|()| log::set_max_level(LevelFilter::Info))
}

#[derive(Default, Debug)]
struct Foo {
    x: i32,
}

fn main() {
    init();

    info!("hi");
    error!("hi");

    info!("hi");
}
