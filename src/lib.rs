use chrono::prelude::*;
use crossterm::{
    execute,
    style::Color,
    style::{Colors, Print, ResetColor, SetColors},
};
use log::{Level, LevelFilter, Metadata, Record, SetLoggerError};
#[cfg(feature = "json")]
use serde::Serialize;
#[cfg(feature = "async")]
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{
    atomic::{AtomicI32, AtomicI64, Ordering::Relaxed},
    LazyLock,
};
#[cfg(feature = "json")]
use serde_json::value::Value;

struct RichLogger {
    last_second: AtomicI64,
    cursor_pos: AtomicI32,
    #[cfg(feature = "async")]
    sender: Sender<RichLoggerRecord>,
}

struct RichLoggerRecord {
    file_name: String,
    level: Level,
    content: String,
}

#[cfg(feature = "json")]
fn wrap_print_json(text: &str, color: Option<Colors>) {
    let width = crossterm::terminal::size().map(|ws| ws.0).unwrap_or(80);

    if LOGGER.cursor_pos.load(Relaxed) as usize + text.len() > width as usize {
        LOGGER.write_string(
            &text[0..(width as usize - LOGGER.cursor_pos.load(Relaxed) as usize)],
            color,
        );
        LOGGER.add_newline();
        LOGGER.pad_to_column(LOGGER.tab_stop(TabStop::Content));
        wrap_print_json(&text[width as usize..], color);
    } else {
        LOGGER.write_string(&text, color);
    }
}

#[cfg(feature = "json")]
fn json_impl<T: Serialize>(value: &T, level: Level) {
    match serde_json::value::to_value(value).unwrap() {
        Value::Null => wrap_print_json(
            "null",
            Some(Colors {
                foreground: Some(Color::Magenta),
                background: None,
            }),
        ),
        Value::Bool(b) => wrap_print_json(
            if b { "true" } else { "false" },
            Some(Colors {
                foreground: Some(Color::Magenta),
                background: None,
            }),
        ),
        Value::Number(n) => wrap_print_json(
            &n.to_string(),
            Some(Colors {
                foreground: Some(Color::Magenta),
                background: None,
            }),
        ),
        Value::String(s) => {
            wrap_print_json(
                r#"""#,
                Some(Colors {
                    foreground: Some(Color::Green),
                    background: None,
                }),
            );
            wrap_print_json(
                &s,
                Some(Colors {
                    foreground: Some(Color::Green),
                    background: None,
                }),
            );
            wrap_print_json(
                r#"""#,
                Some(Colors {
                    foreground: Some(Color::Green),
                    background: None,
                }),
            );
        }
        Value::Array(a) => {
            wrap_print_json(
                "[",
                Some(Colors {
                    foreground: Some(Color::White),
                    background: None,
                }),
            );
            for (i, v) in a.iter().enumerate() {
                if i > 0 {
                    wrap_print_json(
                        ", ",
                        Some(Colors {
                            foreground: Some(Color::White),
                            background: None,
                        }),
                    );
                }
                json_impl(v, level);
            }
            wrap_print_json(
                "]",
                Some(Colors {
                    foreground: Some(Color::White),
                    background: None,
                }),
            );
        }
        Value::Object(o) => {
            wrap_print_json(
                "{",
                Some(Colors {
                    foreground: Some(Color::White),
                    background: None,
                }),
            );
            for (i, (k, v)) in o.iter().enumerate() {
                if i > 0 {
                    wrap_print_json(
                        ", ",
                        Some(Colors {
                            foreground: Some(Color::White),
                            background: None,
                        }),
                    );
                }
                wrap_print_json(
                    r#"""#,
                    Some(Colors {
                        foreground: Some(Color::Green),
                        background: None,
                    }),
                );
                wrap_print_json(
                    &format!("{}", k),
                    Some(Colors {
                        foreground: Some(Color::Green),
                        background: None,
                    }),
                );
                wrap_print_json(
                    r#"""#,
                    Some(Colors {
                        foreground: Some(Color::Green),
                        background: None,
                    }),
                );
                wrap_print_json(
                    ": ",
                    Some(Colors {
                        foreground: Some(Color::White),
                        background: None,
                    }),
                );
                json_impl(v, level);
            }
            wrap_print_json(
                "}",
                Some(Colors {
                    foreground: Some(Color::White),
                    background: None,
                }),
            );
        }
    }
}

#[cfg(feature = "json")]
pub fn json<T: Serialize>(value: &T, level: Level) {
    let self_log = &*LOGGER;
    self_log.pad_to_column(self_log.tab_stop(TabStop::Time));
    self_log.write_time();
    self_log.pad_to_column(self_log.tab_stop(TabStop::Level));
    self_log.write_level(level);
    self_log.pad_to_column(self_log.tab_stop(TabStop::Content));

    json_impl(value, level);
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
            content: value.args().to_string(),
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

fn log_impl(record: RichLoggerRecord) {
    let self_log = &*LOGGER;
    let width = crossterm::terminal::size().map(|ws| ws.0).unwrap_or(80);
    self_log.pad_to_column(self_log.tab_stop(TabStop::Time));
    self_log.write_time();
    self_log.pad_to_column(self_log.tab_stop(TabStop::Level));
    self_log.write_level(record.level);
    let lines = record
        .content
        .replace("\t", "    ")
        .replace("\r", "")
        .chars()
        .collect::<Vec<_>>()
        .chunks(
            width as usize
                - self_log.tab_stop(TabStop::Content) as usize
                - record.file_name.len()
                - 1,
        )
        .map(|chunk| chunk.iter().collect::<String>())
        .map(|text| {
            text.split("\n")
                .map(|t| t.to_owned())
                .collect::<Vec<String>>()
        })
        .flatten()
        .collect::<Vec<String>>();

    let mut first_line = true;

    for line in lines {
        self_log.pad_to_column(self_log.tab_stop(TabStop::Content));
        self_log.write_string(&line, None);
        if first_line {
            self_log.pad_to_column((width as usize - record.file_name.len()) as i32);
            self_log.write_string(
                &record.file_name,
                Some(Colors {
                    foreground: Some(Color::Grey),
                    background: None,
                }),
            );
            first_line = false;
        }

        self_log.add_newline();
    }
}

impl log::Log for RichLogger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    #[cfg(feature = "async")]
    fn log(&self, record: &Record) {
        let _ignore = self.sender.send((*record).clone().into());
    }

    #[cfg(not(feature = "async"))]
    fn log(&self, record: &Record) {
        log_impl((*record).clone().into());
    }

    fn flush(&self) {}
}

#[cfg(feature = "async")]
fn spawn_logger_thread(rx: Receiver<RichLoggerRecord>) {
    std::thread::spawn(move || loop {
        if let Ok(msg) = rx.recv() {
            log_impl(msg);
        } else {
            break;
        }
    });
}

static LOGGER: LazyLock<RichLogger> = LazyLock::new(|| {
    #[cfg(feature = "async")]
    let (tx, rx) = mpsc::channel::<RichLoggerRecord>();
    #[cfg(feature = "async")]
    spawn_logger_thread(rx);
    RichLogger {
        #[cfg(feature = "async")]
        sender: tx,
        last_second: AtomicI64::default(),
        cursor_pos: AtomicI32::default(),
    }
});

pub fn init(level: LevelFilter) -> Result<(), SetLoggerError> {
    log::set_logger(&*LOGGER).map(|()| log::set_max_level(level))
}
