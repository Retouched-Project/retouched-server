// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 ddavef/KinteLiX retouched-server

use core::pin::Pin;
use cxx_qt_lib::QString;
use std::path::PathBuf;
use std::sync::{LazyLock, Mutex};

use crate::gui::server_backend::BACKEND_INIT;
use crate::touchy_patcher::{self, PatchStep, ToolStatus};

struct PatcherInternalState {
    initialized: bool,
    data_dir: PathBuf,
    step: touchy_patcher::SharedStep,
    tool_status: Option<ToolStatus>,
    patch_thread: Option<std::thread::JoinHandle<()>>,
}

static PATCHER_STATE: LazyLock<Mutex<PatcherInternalState>> = LazyLock::new(|| {
    Mutex::new(PatcherInternalState {
        initialized: false,
        data_dir: PathBuf::new(),
        step: touchy_patcher::new_shared_step(),
        tool_status: None,
        patch_thread: None,
    })
});

fn ensure_initialized(state: &mut PatcherInternalState) {
    if state.initialized {
        return;
    }
    if let Some(init) = BACKEND_INIT.get() {
        state.data_dir = crate::app_dirs::tools_cache_dir(init.data_dir.as_deref());
        state.tool_status = Some(ToolStatus::detect(&state.data_dir));
        state.initialized = true;
    }
}

pub struct PatcherBackendRust {
    apk_path: QString,
    target_ip: QString,
    current_step: QString,
    step_is_error: bool,
    step_is_done: bool,
    step_is_working: bool,
    output_path: QString,
    apktool_ok: bool,
    jadx_ok: bool,
    uber_ok: bool,
    jre_ok: bool,
    all_tools_present: bool,
    is_busy: bool,
}

impl Default for PatcherBackendRust {
    fn default() -> Self {
        Self {
            apk_path: QString::from(""),
            target_ip: QString::from(""),
            current_step: QString::from("Idle"),
            step_is_error: false,
            step_is_done: false,
            step_is_working: false,
            output_path: QString::from(""),
            apktool_ok: false,
            jadx_ok: false,
            uber_ok: false,
            jre_ok: false,
            all_tools_present: false,
            is_busy: false,
        }
    }
}

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qproperty(QString, apk_path)]
        #[qproperty(QString, target_ip)]
        #[qproperty(QString, current_step)]
        #[qproperty(bool, step_is_error)]
        #[qproperty(bool, step_is_done)]
        #[qproperty(bool, step_is_working)]
        #[qproperty(QString, output_path)]
        #[qproperty(bool, apktool_ok)]
        #[qproperty(bool, jadx_ok)]
        #[qproperty(bool, uber_ok)]
        #[qproperty(bool, jre_ok)]
        #[qproperty(bool, all_tools_present)]
        #[qproperty(bool, is_busy)]
        type PatcherBackend = super::PatcherBackendRust;

        #[qinvokable]
        fn refresh(self: Pin<&mut PatcherBackend>);

        #[qinvokable]
        fn set_apk_path_value(self: Pin<&mut PatcherBackend>, path: QString);

        #[qinvokable]
        fn set_target_ip_value(self: Pin<&mut PatcherBackend>, ip: QString);

        #[qinvokable]
        fn download_tools(self: Pin<&mut PatcherBackend>);

        #[qinvokable]
        fn patch_and_sign(self: Pin<&mut PatcherBackend>);

        #[qinvokable]
        fn open_output_folder(self: &PatcherBackend);
    }
}

impl qobject::PatcherBackend {
    fn refresh(mut self: Pin<&mut Self>) {
        let mut state = PATCHER_STATE.lock().unwrap();
        ensure_initialized(&mut state);

        if let Some(ref handle) = state.patch_thread {
            if handle.is_finished() {
                state.tool_status = Some(ToolStatus::detect(&state.data_dir));
            }
        }

        let is_busy = state
            .patch_thread
            .as_ref()
            .is_some_and(|h| !h.is_finished());
        self.as_mut().set_is_busy(is_busy);

        let step = state.step.lock().unwrap().clone();
        self.as_mut()
            .set_current_step(QString::from(&format!("{}", step)));
        self.as_mut()
            .set_step_is_error(matches!(step, PatchStep::Error(_)));
        self.as_mut()
            .set_step_is_done(matches!(step, PatchStep::Done(_)));
        self.as_mut().set_step_is_working(!matches!(
            step,
            PatchStep::Idle | PatchStep::Done(_) | PatchStep::Error(_)
        ));

        if let PatchStep::Done(ref p) = step {
            self.as_mut()
                .set_output_path(QString::from(&p.display().to_string()));
        }

        if let Some(ref ts) = state.tool_status {
            self.as_mut().set_apktool_ok(ts.apktool.is_some());
            self.as_mut().set_jadx_ok(ts.jadx.is_some());
            self.as_mut().set_uber_ok(ts.uber.is_some());
            self.as_mut().set_jre_ok(ts.jre.is_some());
            self.as_mut().set_all_tools_present(
                ts.apktool.is_some() && ts.jadx.is_some() && ts.uber.is_some() && ts.jre.is_some(),
            );
        }

        if let Some(init) = BACKEND_INIT.get() {
            let ip = init.shared.detected_lan_ip();
            if self.as_ref().target_ip().to_string().is_empty() {
                self.set_target_ip(QString::from(&ip));
            }
        }
    }

    fn set_apk_path_value(self: Pin<&mut Self>, path: QString) {
        self.set_apk_path(path);
    }

    fn set_target_ip_value(self: Pin<&mut Self>, ip: QString) {
        self.set_target_ip(ip);
    }

    fn download_tools(self: Pin<&mut Self>) {
        let mut state = PATCHER_STATE.lock().unwrap();
        ensure_initialized(&mut state);

        let data_dir = state.data_dir.clone();
        let step = state.step.clone();

        state.patch_thread = Some(std::thread::spawn(move || {
            *step.lock().unwrap() = PatchStep::DownloadingTools;
            match touchy_patcher::ensure_tools(&data_dir) {
                Ok(_) => *step.lock().unwrap() = PatchStep::Idle,
                Err(e) => *step.lock().unwrap() = PatchStep::Error(e.to_string()),
            }
        }));
    }

    fn patch_and_sign(self: Pin<&mut Self>) {
        let mut state = PATCHER_STATE.lock().unwrap();
        ensure_initialized(&mut state);

        let apk_str = self.apk_path().to_string();
        let ip_str = self.target_ip().to_string();

        if apk_str.is_empty() || ip_str.is_empty() {
            return;
        }

        let apk = PathBuf::from(apk_str);
        let data_dir = state.data_dir.clone();
        let step = state.step.clone();

        state.patch_thread = Some(std::thread::spawn(move || {
            if let Err(e) = touchy_patcher::run_patch_pipeline(&apk, &ip_str, &data_dir, &step) {
                *step.lock().unwrap() = PatchStep::Error(e.to_string());
            }
        }));
    }

    fn open_output_folder(&self) {
        let path_str = self.output_path().to_string();
        if !path_str.is_empty() {
            let path = std::path::Path::new(&path_str);
            if let Some(parent) = path.parent() {
                let _ = open::that(parent);
            }
        }
    }
}
