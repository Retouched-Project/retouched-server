// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 ddavef/KinteLiX retouched-server

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Dialogs
import com.retouched.server

Item {
    id: settingsTab

    property var trustEntries: []
    property var hostsEntries: []

    SettingsBackend {
        id: backend
    }

    Timer {
        interval: 1000
        running: true
        repeat: true
        onTriggered: {
            backend.refresh();
            try {
                settingsTab.trustEntries = JSON.parse(backend.trust_entries_json);
            } catch (e) {
                settingsTab.trustEntries = [];
            }
            try {
                settingsTab.hostsEntries = JSON.parse(backend.hosts_status_json);
            } catch (e) {
                settingsTab.hostsEntries = [];
            }
        }
    }

    FolderDialog {
        id: trustDirDialog
        title: "Select directory to trust"
        onAccepted: {
            backend.set_new_trust_dir(selectedFolder.toString().replace("file://", ""));
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
                text: "Flash Player Trust"
                font.bold: true
                font.pixelSize: 16
                Layout.leftMargin: 8
            }

            Label {
                text: settingsTab.trustEntries.length === 0 ? "No trusted directories configured." : "Trusted directories:"
                Layout.leftMargin: 8
                font.bold: settingsTab.trustEntries.length > 0
            }

            Repeater {
                model: settingsTab.trustEntries
                RowLayout {
                    Layout.leftMargin: 16
                    spacing: 8
                    Label {
                        text: modelData
                        font.family: "monospace"
                    }
                    Button {
                        text: "Remove"
                        flat: true
                        onClicked: backend.remove_trust_dir(index)
                    }
                }
            }

            RowLayout {
                Layout.leftMargin: 8
                spacing: 8

                Label {
                    text: "Add directory:"
                }
                TextField {
                    text: backend.new_trust_directory
                    Layout.fillWidth: true
                    onEditingFinished: backend.set_new_trust_dir(text)
                }
                Button {
                    text: "Browse..."
                    onClicked: trustDirDialog.open()
                }
            }

            RowLayout {
                Layout.leftMargin: 8
                spacing: 8

                Button {
                    text: "Add to trust config"
                    enabled: backend.new_trust_directory !== ""
                    onClicked: backend.add_trust_dir()
                }
                Button {
                    text: "Remove all"
                    onClicked: backend.remove_all_trust()
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
                text: "Hosts File Redirect"
                font.bold: true
                font.pixelSize: 16
                Layout.leftMargin: 8
            }

            RowLayout {
                Layout.leftMargin: 8
                spacing: 8

                Label {
                    text: "Redirect IP:"
                }
                TextField {
                    text: backend.hosts_redirect_ip
                    implicitWidth: 200
                    onEditingFinished: backend.set_hosts_ip(text)
                }
            }

            Repeater {
                model: settingsTab.hostsEntries
                RowLayout {
                    Layout.leftMargin: 16
                    spacing: 8
                    Label {
                        text: modelData.status === "ok" ? "[OK]" : "[--]"
                        color: modelData.status === "ok" ? "#00c800" : "#c80000"
                        font.family: "monospace"
                    }
                    Label {
                        text: modelData.status === "ok" ? modelData.domain + " -> " + modelData.ip : modelData.domain + " -- not configured"
                    }
                }
            }

            RowLayout {
                Layout.leftMargin: 8
                spacing: 8

                Button {
                    text: "Apply hosts redirect"
                    onClicked: backend.apply_hosts_redirect()
                }
                Button {
                    text: "Remove hosts redirect"
                    onClicked: backend.remove_hosts_redirect()
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
                text: "Firewall"
                font.bold: true
                font.pixelSize: 16
                Layout.leftMargin: 8
            }

            Label {
                text: "Backend: " + backend.firewall_backend
                Layout.leftMargin: 8
            }

            Label {
                text: "Required ports:\nTCP 8080 (HTTP Server)\nTCP 8088 (BM Registry)\nTCP 8089 (Retouched Web)\nTCP 8443 (WebRTC Bridge)\nTCP 9081 (Game)"
                Layout.leftMargin: 8
                opacity: 0.7
            }

            Label {
                text: "No supported firewall manager detected. Open ports manually."
                color: "#ffc800"
                visible: backend.firewall_backend === "None"
                Layout.leftMargin: 8
            }

            RowLayout {
                Layout.leftMargin: 8
                spacing: 8
                visible: backend.firewall_backend !== "None" && backend.firewall_backend !== ""

                Button {
                    text: "Open ports"
                    onClicked: backend.open_ports()
                }
                Button {
                    text: "Close ports"
                    onClicked: backend.close_ports()
                }
            }

            Item {
                Layout.fillHeight: true
            }
        }
    }
}
