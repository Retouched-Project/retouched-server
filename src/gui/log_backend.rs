// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 ddavef/KinteLiX retouched-server

use cxx_qt_lib::QString;
use std::time::Instant;

use crate::gui::server_backend::BACKEND_INIT;

#[derive(Default)]
pub struct LogBackendRust;

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        type LogBackend = super::LogBackendRust;

        #[qinvokable]
        fn log_entries_json(self: &LogBackend, level_filter: i32) -> QString;

        #[qinvokable]
        fn clear_log(self: &LogBackend);

        #[qinvokable]
        fn capture_level(self: &LogBackend) -> i32;

        #[qinvokable]
        fn set_capture_level(self: &LogBackend, level: i32);
    }
}

fn level_filter_from_i32(level: i32) -> log::LevelFilter {
    match level {
        1 => log::LevelFilter::Error,
        2 => log::LevelFilter::Warn,
        3 => log::LevelFilter::Info,
        4 => log::LevelFilter::Debug,
        5 => log::LevelFilter::Trace,
        _ => log::LevelFilter::Info,
    }
}

impl qobject::LogBackend {
    fn log_entries_json(&self, level_filter: i32) -> QString {
        let Some(init) = BACKEND_INIT.get() else {
            return QString::from("[]");
        };
        let filter = level_filter_from_i32(level_filter);

        let log_buf = init.shared.log_buffer.lock().unwrap();
        let start_time = log_buf
            .entries()
            .front()
            .map(|e| e.timestamp)
            .unwrap_or_else(Instant::now);

        let entries: Vec<_> = log_buf
            .entries()
            .iter()
            .filter(|e| e.level <= filter)
            .map(|e| {
                let elapsed = e.timestamp.duration_since(start_time).as_secs_f64();
                let color = match e.level {
                    log::Level::Error => "#ff5555",
                    log::Level::Warn => "#ffcc00",
                    log::Level::Info => "#5fd75f",
                    log::Level::Debug => "#5f9fff",
                    log::Level::Trace => "#5fd7d7",
                };
                serde_json::json!({
                    "time": format!("{:.3}", elapsed),
                    "level": format!("{}", e.level),
                    "message": &e.message,
                    "color": color,
                })
            })
            .collect();

        QString::from(&serde_json::Value::Array(entries).to_string())
    }

    fn clear_log(&self) {
        if let Some(init) = BACKEND_INIT.get() {
            init.shared.log_buffer.lock().unwrap().clear();
        }
    }

    fn capture_level(&self) -> i32 {
        match log::max_level() {
            log::LevelFilter::Off => 0,
            log::LevelFilter::Error => 1,
            log::LevelFilter::Warn => 2,
            log::LevelFilter::Info => 3,
            log::LevelFilter::Debug => 4,
            log::LevelFilter::Trace => 5,
        }
    }

    fn set_capture_level(&self, level: i32) {
        log::set_max_level(level_filter_from_i32(level));
    }
}
