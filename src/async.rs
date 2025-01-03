#[cfg(feature = "json")]
use crate::{json::tokenize_json_value, ContentType};
use crate::{log_impl, RichLoggerRecord};
use log::{Metadata, Record};
use std::{
    sync::mpsc,
    sync::mpsc::{Receiver, Sender},
    sync::{
        atomic::{AtomicI32, AtomicI64},
        LazyLock,
    },
};

pub(crate) struct RichLogger {
    pub last_second: AtomicI64,
    pub cursor_pos: AtomicI32,
    pub sender: Sender<RichLoggerRecord>,
}

pub(crate) fn spawn_logger_thread(rx: Receiver<RichLoggerRecord>) {
    std::thread::spawn(move || loop {
        if let Ok(msg) = rx.recv() {
            log_impl(msg);
        } else {
            break;
        }
    });
}

impl log::Log for RichLogger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    #[cfg(feature = "json")]
    fn log(&self, record: &Record) {
        let gg: Result<serde_json::Value, serde_json::Error> =
            serde_json::from_str(&record.args().to_string());

        match gg {
            Ok(g) => {
                let text_record = (*record).clone().into();
                let _ignore = self.sender.send(RichLoggerRecord {
                    content: ContentType::JsonContent(tokenize_json_value(&g)),
                    ..text_record
                });
            }
            Err(_) => {
                let _ignore = self.sender.send((*record).clone().into());
            }
        }
    }

    #[cfg(not(feature = "json"))]
    fn log(&self, record: &Record) {
        let _ignore = self.sender.send((*record).clone().into());
    }

    fn flush(&self) {}
}

pub(crate) static LOGGER: LazyLock<RichLogger> = LazyLock::new(|| {
    let (tx, rx) = mpsc::channel::<RichLoggerRecord>();
    spawn_logger_thread(rx);
    RichLogger {
        sender: tx,
        last_second: AtomicI64::default(),
        cursor_pos: AtomicI32::default(),
    }
});
