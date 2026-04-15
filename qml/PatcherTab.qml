// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 ddavef/KinteLiX retouched-server

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Dialogs
import com.retouched.server

Item {
    id: patcherTab

    PatcherBackend {
        id: backend
    }

    Timer {
        interval: 500
        running: true
        repeat: true
        onTriggered: backend.refresh()
    }

    FileDialog {
        id: apkDialog
        title: "Select APK file"
        nameFilters: ["APK files (*.apk)"]
        onAccepted: {
            backend.set_apk_path_value(selectedFile.toString());
        }
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 8
        spacing: 8

        Label {
            text: "Touchy Patcher"
            font.bold: true
            font.pixelSize: 16
        }

        RowLayout {
            spacing: 8

            Label {
                text: "APK Path:"
            }
            TextField {
                text: backend.apk_path
                Layout.fillWidth: true
                onEditingFinished: backend.set_apk_path_value(text)
            }
            Button {
                text: "Browse..."
                onClicked: apkDialog.open()
            }
        }

        RowLayout {
            spacing: 8

            Label {
                text: "Target IP:"
            }
            TextField {
                text: backend.target_ip
                implicitWidth: 200
                onEditingFinished: backend.set_target_ip_value(text)
            }
        }

        Rectangle {
            Layout.fillWidth: true
            height: 1
            color: palette.mid
        }

        Label {
            text: "Tools"
            font.bold: true
            font.pixelSize: 14
        }

        RowLayout {
            spacing: 8
            Label {
                text: backend.apktool_ok ? "[OK]" : "[--]"
                color: backend.apktool_ok ? "#00c800" : "#c80000"
                font.family: "monospace"
            }
            Label {
                text: "apktool"
            }
        }

        RowLayout {
            spacing: 8
            Label {
                text: backend.jadx_ok ? "[OK]" : "[--]"
                color: backend.jadx_ok ? "#00c800" : "#c80000"
                font.family: "monospace"
            }
            Label {
                text: "jadx"
            }
        }

        RowLayout {
            spacing: 8
            Label {
                text: backend.uber_ok ? "[OK]" : "[--]"
                color: backend.uber_ok ? "#00c800" : "#c80000"
                font.family: "monospace"
            }
            Label {
                text: "uber-apk-signer"
            }
        }

        RowLayout {
            spacing: 8
            Label {
                text: backend.jre_ok ? "[OK]" : "[--]"
                color: backend.jre_ok ? "#00c800" : "#c80000"
                font.family: "monospace"
            }
            Label {
                text: "adoptium jre-17"
            }
        }

        Button {
            text: "Download Tools"
            enabled: !backend.is_busy && !backend.all_tools_present
            onClicked: backend.download_tools()
        }

        Rectangle {
            Layout.fillWidth: true
            height: 1
            color: palette.mid
        }

        Button {
            text: "Patch & Sign APK"
            enabled: !backend.is_busy && backend.all_tools_present && backend.apk_path !== "" && backend.target_ip !== ""
            onClicked: backend.patch_and_sign()
        }

        RowLayout {
            spacing: 8
            visible: backend.step_is_working

            BusyIndicator {
                implicitWidth: 24
                implicitHeight: 24
            }
            Label {
                text: backend.current_step
            }
        }

        Label {
            text: backend.current_step
            color: "#00c800"
            visible: backend.step_is_done
        }

        RowLayout {
            visible: backend.step_is_done
            spacing: 8

            Button {
                text: "Open output folder"
                onClicked: backend.open_output_folder()
            }
        }

        Label {
            text: backend.current_step
            color: "#ff5050"
            visible: backend.step_is_error
            wrapMode: Text.Wrap
            Layout.fillWidth: true
        }

        Item {
            Layout.fillHeight: true
        }
    }
}
