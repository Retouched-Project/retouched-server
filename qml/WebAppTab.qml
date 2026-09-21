// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 ddavef/KinteLiX retouched-server

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Dialogs
import com.retouched.server

Item {
    id: webTab

    WebAppBackend {
        id: backend
    }

    Timer {
        interval: 200
        running: true
        repeat: true
        onTriggered: backend.refresh()
    }

    FolderDialog {
        id: customDirDialog
        title: "Select Retouched Web directory"
        onAccepted: {
            backend.set_custom_dir(selectedFolder.toString());
        }
    }

    ScrollView {
        anchors.fill: parent
        contentWidth: availableWidth

        ColumnLayout {
            width: parent.width
            spacing: 8

            Item {
                Layout.preferredHeight: 8
            }

            Label {
                text: "WebRTC Bridge"
                font.bold: true
                font.pixelSize: 16
                Layout.leftMargin: 8
            }

            RowLayout {
                Layout.leftMargin: 8
                spacing: 8

                StatusIndicator {
                    status: backend.bridge_status.toLowerCase()
                    Layout.alignment: Qt.AlignVCenter
                }

                Label {
                    text: backend.bridge_status
                    color: {
                        switch (backend.bridge_status) {
                        case "Running":
                            return "#00c800";
                        case "Error":
                            return "#ff5050";
                        case "Starting":
                            return "#c8c800";
                        default:
                            return palette.text;
                        }
                    }
                }
            }

            Label {
                text: backend.bridge_error
                color: "#ff5050"
                visible: backend.bridge_error !== ""
                Layout.leftMargin: 8
                wrapMode: Text.Wrap
                Layout.fillWidth: true
            }

            RowLayout {
                Layout.leftMargin: 8
                spacing: 8

                Label {
                    text: "Bridge Port:"
                }
                TextField {
                    text: backend.bridge_port
                    enabled: backend.bridge_status === "Stopped" || backend.bridge_status === "Error"
                    implicitWidth: 80
                    inputMethodHints: Qt.ImhDigitsOnly
                    validator: IntValidator { bottom: 1; top: 65535 }
                    onEditingFinished: backend.set_bridge_port_value(text)
                }

                Button {
                    text: "↺"
                    font.pixelSize: 16
                    implicitWidth: 34
                    enabled: backend.bridge_port !== "8443"
                             && (backend.bridge_status === "Stopped" || backend.bridge_status === "Error")
                    onClicked: backend.restore_default_bridge_port()
                    ToolTip.visible: hovered
                    ToolTip.text: "Restore default port"
                }

            }

            RowLayout {
                Layout.leftMargin: 8
                spacing: 8

                CheckBox {
                    id: localRegistry
                    text: "Bridge to the server in this app"
                    checked: backend.use_local_registry
                    enabled: backend.bridge_status === "Stopped" || backend.bridge_status === "Error"
                    onToggled: backend.set_use_local_registry_value(checked)
                }

                Label {
                    text: "Registry:"
                    enabled: !localRegistry.checked
                }
                TextField {
                    text: backend.registry_host
                    placeholderText: "host or host:port"
                    enabled: !localRegistry.checked
                             && (backend.bridge_status === "Stopped" || backend.bridge_status === "Error")
                    implicitWidth: 180
                    onEditingFinished: backend.set_registry_host_value(text)
                }
            }

            Label {
                text: localRegistry.checked
                      ? "The bridge waits for this app's server and connects the moment it starts."
                      : "The bridge retries on a timer, since a server elsewhere announces nothing."
                color: "#aaa"
                font.pixelSize: 11
                Layout.leftMargin: 8
                Layout.fillWidth: true
                wrapMode: Text.Wrap
            }

            RowLayout {
                Layout.leftMargin: 8
                spacing: 8

                Button {
                    text: backend.bridge_status === "Running" ? "Stop Bridge" : "Start Bridge"
                    enabled: backend.bridge_status === "Stopped" || backend.bridge_status === "Error" || backend.bridge_status === "Running"
                    onClicked: {
                        if (backend.bridge_status === "Running")
                            backend.stop_bridge();
                        else
                            backend.start_bridge();
                        backend.refresh();
                    }
                }
            }

            Rectangle {
                Layout.fillWidth: true
                height: 1
                color: palette.mid
                Layout.topMargin: 4
                Layout.bottomMargin: 4
            }

            Label {
                text: "Retouched Web"
                font.bold: true
                font.pixelSize: 16
                Layout.leftMargin: 8
            }

            Label {
                text: "Download directory: " + backend.default_web_dir
                Layout.leftMargin: 8
                opacity: 0.7
            }

            RowLayout {
                Layout.leftMargin: 8
                spacing: 8

                Label {
                    text: "Custom directory:"
                }
                TextField {
                    text: backend.custom_web_dir
                    Layout.fillWidth: true
                    readOnly: true
                }
                Button {
                    text: "Browse..."
                    onClicked: customDirDialog.open()
                }
                Button {
                    text: "Clear"
                    visible: backend.custom_web_dir !== ""
                    onClicked: { backend.clear_custom_dir(); backend.refresh(); }
                }
            }

            Label {
                visible: backend.custom_web_dir !== ""
                text: backend.has_package_json ? "Custom directory: Retouched Web found" : "Custom directory: Retouched Web not found"
                color: backend.has_package_json ? "#00c800" : "#ffc800"
                Layout.leftMargin: 8
            }

            RowLayout {
                Layout.leftMargin: 8
                spacing: 8

                StatusIndicator {
                    status: {
                        switch (backend.web_app_status) {
                        case "Running":
                            return "running";
                        case "Error":
                            return "error";
                        case "Downloading...":
                        case "Installing...":
                            return "starting";
                        default:
                            return "stopped";
                        }
                    }
                    Layout.alignment: Qt.AlignVCenter
                }

                Label {
                    text: backend.web_app_status
                    color: {
                        switch (backend.web_app_status) {
                        case "Running":
                            return "#00c800";
                        case "Error":
                            return "#ff5050";
                        default:
                            return palette.text;
                        }
                    }
                }

                Label {
                    text: "(" + backend.web_app_version + ")"
                    visible: backend.web_app_version !== ""
                    opacity: 0.7
                }
            }

            Label {
                Layout.leftMargin: 8
                visible: backend.custom_web_dir === "" && backend.web_app_version !== "" && backend.latest_version !== ""
                text: backend.update_available ? ("Update available: " + backend.latest_version) : "Up to date"
                color: backend.update_available ? "#ffc800" : "#00c800"
            }

            Label {
                text: backend.web_app_error
                color: "#ff5050"
                visible: backend.web_app_error !== ""
                Layout.leftMargin: 8
                wrapMode: Text.Wrap
                Layout.fillWidth: true
            }

            RowLayout {
                Layout.leftMargin: 8
                spacing: 8

                Button {
                    text: (backend.web_app_status === "Not found" || backend.web_app_status === "Error") ? "Download Retouched Web" : "Update Retouched Web"
                    enabled: backend.web_app_status !== "Downloading..." && backend.web_app_status !== "Running"
                    highlighted: backend.update_available
                    onClicked: { backend.download_release(); backend.refresh(); }
                }

                Button {
                    text: "Check for Updates"
                    enabled: backend.web_app_status !== "Downloading..."
                    onClicked: { backend.check_for_updates(); backend.refresh(); }
                }

                BusyIndicator {
                    visible: backend.web_app_status === "Downloading..." || backend.web_app_status === "Installing..."
                    implicitWidth: 24
                    implicitHeight: 24
                }
            }

            RowLayout {
                Layout.leftMargin: 8
                spacing: 8

                Button {
                    text: backend.web_app_status === "Running" ? "Stop Retouched Web" : "Start Retouched Web"
                    enabled: (backend.web_app_status === "Running") || (backend.bridge_status === "Running" && (backend.web_app_status === "Ready" || backend.web_app_status === "Error"))
                    onClicked: {
                        if (backend.web_app_status === "Running")
                            backend.stop_web_app();
                        else
                            backend.start_web_app();
                        backend.refresh();
                    }
                }
            }

            Label {
                text: "Start the bridge first before launching Retouched Web."
                visible: backend.bridge_status !== "Running" && backend.web_app_status !== "Running"
                opacity: 0.5
                Layout.leftMargin: 8
            }

            Rectangle {
                Layout.fillWidth: true
                height: 1
                color: palette.mid
                Layout.topMargin: 4
                Layout.bottomMargin: 4
            }

            ColumnLayout {
                visible: backend.web_app_status === "Running"
                Layout.leftMargin: 8
                spacing: 8

                Label {
                    text: "Access URL"
                    font.bold: true
                    font.pixelSize: 16
                }

                Label {
                    text: backend.web_url
                    font.family: "monospace"
                }

                Label {
                    text: "Scan with your phone:"
                }

                RowLayout {
                    spacing: 20

                    ColumnLayout {
                        Label {
                            text: "Retouched Web"
                        }
                        Image {
                            source: backend.qr_web_url
                            sourceSize.width: 200
                            sourceSize.height: 200
                            visible: backend.qr_web_url !== ""
                        }
                    }

                    ColumnLayout {
                        Label {
                            text: "Onboarding Page"
                        }
                        Image {
                            source: backend.qr_onboard_url
                            sourceSize.width: 200
                            sourceSize.height: 200
                            visible: backend.qr_onboard_url !== ""
                        }
                    }
                }
            }

            Item {
                Layout.fillHeight: true
            }
        }
    }
}
