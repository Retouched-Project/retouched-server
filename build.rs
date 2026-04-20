// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 ddavef/KinteLiX retouched-server

#[cfg(feature = "gui")]
fn main() {
    use cxx_qt_build::{CxxQtBuilder, QmlModule};

    CxxQtBuilder::new_qml_module(QmlModule::new("com.retouched.server").qml_files([
        "qml/main.qml",
        "qml/ServerTab.qml",
        "qml/WebAppTab.qml",
        "qml/PatcherTab.qml",
        "qml/SettingsTab.qml",
        "qml/AboutTab.qml",
        "qml/SetupWizard.qml",
        "qml/components/StatusIndicator.qml",
        "qml/components/DeviceCard.qml",
        "qml/components/LogViewer.qml",
    ]))
    .qrc("assets/assets.qrc")
    .file("src/gui/server_backend.rs")
    .file("src/gui/web_app_backend.rs")
    .file("src/gui/patcher_backend.rs")
    .file("src/gui/settings_backend.rs")
    .file("src/gui/wizard_backend.rs")
    .qt_module("Quick")
    .qt_module("QuickControls2")
    .qt_module("Widgets")
    .cpp_file("cpp/tray_manager.h")
    .cpp_file("cpp/tray_manager.cpp")
    .cpp_file("cpp/qt_app.cpp")
    .build();

    #[cfg(target_os = "windows")]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/retouched_logo_icons.ico");
        res.compile().unwrap();
    }

}

#[cfg(not(feature = "gui"))]
fn main() {}
