// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 ddavef/KinteLiX retouched-server

use log::{LevelFilter, Log, Metadata, Record};
use std::sync::Arc;

use crate::logging::{DEP_MAX_LEVEL, is_own_target};
use crate::shared_state::SharedState;

pub struct GuiLogger {
    shared: Arc<SharedState>,
}

impl GuiLogger {
    pub fn init(shared: Arc<SharedState>, level: LevelFilter) {
        let logger = Self { shared };
        log::set_boxed_logger(Box::new(logger)).expect("Failed to set logger");
        log::set_max_level(level);
    }
}

impl Log for GuiLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        let ceiling = if is_own_target(metadata.target()) {
            log::max_level()
        } else {
            DEP_MAX_LEVEL
        };
        metadata.level() <= ceiling
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        let target = if record.target().is_empty() {
            record.module_path().unwrap_or("?")
        } else {
            record.target()
        };
        eprintln!("[{}] [{}] {}", record.level(), target, record.args());

        self.shared
            .push_log(record.level(), format!("{}", record.args()));
    }

    fn flush(&self) {}
}
