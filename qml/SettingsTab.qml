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
        interval: 200
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
            backend.set_new_trust_dir(selectedFolder.toString());
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
                        onClicked: { backend.remove_trust_dir(index); backend.refresh(); }
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
                    onClicked: { backend.add_trust_dir(); backend.refresh(); }
                }
                Button {
                    text: "Remove all"
                    onClicked: { backend.remove_all_trust(); backend.refresh(); }
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
                    onClicked: { backend.apply_hosts_redirect(); backend.refresh(); }
                }
                Button {
                    text: "Remove hosts redirect"
                    onClicked: { backend.remove_hosts_redirect(); backend.refresh(); }
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
                visible: backend.firewall_backend === "none"
                Layout.leftMargin: 8
            }

            RowLayout {
                Layout.leftMargin: 8
                spacing: 8
                visible: backend.firewall_backend !== "none" && backend.firewall_backend !== ""

                Button {
                    text: "Open ports"
                    onClicked: { backend.open_ports(); backend.refresh(); }
                }
                Button {
                    text: "Close ports"
                    onClicked: { backend.close_ports(); backend.refresh(); }
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
                text: "Policy Port Redirect (843)"
                font.bold: true
                font.pixelSize: 16
                Layout.leftMargin: 8
            }

            Label {
                text: "Redirects the privileged Flash/Unity policy port 843 to the server port, so it can be served without running as root."
                Layout.leftMargin: 8
                Layout.fillWidth: true
                wrapMode: Text.WordWrap
                opacity: 0.7
            }

            Label {
                text: "Backend: " + backend.redirect_backend
                Layout.leftMargin: 8
                visible: backend.redirect_backend !== "" && backend.redirect_backend !== "none"
            }

            Label {
                Layout.leftMargin: 8
                Layout.fillWidth: true
                wrapMode: Text.WordWrap
                visible: backend.redirect_backend !== "" && backend.redirect_backend !== "none"
                text: {
                    switch (backend.policy_redirect_status) {
                    case "active":
                        return "Status: active";
                    case "inactive":
                        return "Status: not active - Unity Web Player games will not connect";
                    default:
                        return "Status: unknown (start the server to verify)";
                    }
                }
                color: {
                    switch (backend.policy_redirect_status) {
                    case "active":
                        return "#00c800";
                    case "inactive":
                        return "#ff5050";
                    default:
                        return palette.text;
                    }
                }
            }

            RowLayout {
                Layout.leftMargin: 8
                spacing: 8
                visible: backend.redirect_backend !== "none" && backend.redirect_backend !== ""

                Button {
                    text: "Enable redirect"
                    onClicked: { backend.apply_policy_redirect(); backend.refresh(); }
                }
                Button {
                    text: "Disable redirect"
                    onClicked: { backend.remove_policy_redirect(); backend.refresh(); }
                }
            }

            Item {
                Layout.fillHeight: true
            }
        }
    }
}
