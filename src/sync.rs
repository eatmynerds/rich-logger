use crate::log_impl;
#[cfg(feature = "json")]
use crate::{file_name, json::print_json_pretty};
use log::{Metadata, Record};
use std::sync::{
    atomic::{AtomicI32, AtomicI64},
    LazyLock,
};

pub(crate) struct RichLogger {
    pub last_second: AtomicI64,
    pub cursor_pos: AtomicI32,
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
                print_json_pretty(&g, file_name(record), record.level());
            }
            Err(_) => {
                log_impl((*record).clone().into());
            }
        }
    }

    #[cfg(not(feature = "json"))]
    fn log(&self, record: &Record) {
        log_impl((*record).clone().into());
    }

    fn flush(&self) {}
}

pub(crate) static LOGGER: LazyLock<RichLogger> = LazyLock::new(|| RichLogger {
    last_second: AtomicI64::default(),
    cursor_pos: AtomicI32::default(),
});
