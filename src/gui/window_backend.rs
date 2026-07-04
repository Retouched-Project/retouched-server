// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 ddavef/KinteLiX retouched-server

use crate::gui::server_backend::BACKEND_INIT;

#[derive(Default)]
pub struct WindowBackendRust;

#[cxx_qt::bridge]
pub mod qobject {
    extern "RustQt" {
        #[qobject]
        #[qml_element]
        type WindowBackend = super::WindowBackendRust;

        #[qinvokable]
        fn saved_width(self: &WindowBackend) -> i32;

        #[qinvokable]
        fn saved_height(self: &WindowBackend) -> i32;

        #[qinvokable]
        fn save_size(self: &WindowBackend, width: i32, height: i32);
    }
}

impl qobject::WindowBackend {
    fn saved_width(&self) -> i32 {
        BACKEND_INIT
            .get()
            .and_then(|init| init.config.lock().unwrap().window_width)
            .map(|w| w as i32)
            .unwrap_or(0)
    }

    fn saved_height(&self) -> i32 {
        BACKEND_INIT
            .get()
            .and_then(|init| init.config.lock().unwrap().window_height)
            .map(|h| h as i32)
            .unwrap_or(0)
    }

    fn save_size(&self, width: i32, height: i32) {
        if width <= 0 || height <= 0 {
            return;
        }
        if let Some(init) = BACKEND_INIT.get() {
            let mut cfg = init.config.lock().unwrap();
            if cfg.window_width == Some(width as u32) && cfg.window_height == Some(height as u32) {
                return;
            }
            cfg.window_width = Some(width as u32);
            cfg.window_height = Some(height as u32);
            if let Err(e) = cfg.save_to_file(&init.config_path) {
                log::warn!("Failed to save window size: {}", e);
            }
        }
    }
}
