// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 ddavef/KinteLiX retouched-server

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtQuick.Dialogs
import com.retouched.server

Dialog {
    id: wizard
    modal: true
    width: 520
    height: 420
    title: "Setup Wizard"
    closePolicy: Popup.NoAutoClose

    property var trustEntries: []
    property var hostsEntries: []

    WizardBackend {
        id: backend
    }

    visible: backend.active

    Timer {
        interval: 1000
        running: backend.active
        repeat: true
        onTriggered: {
            backend.refresh();
            try {
                wizard.trustEntries = JSON.parse(backend.trust_entries_json);
            } catch (e) {
                wizard.trustEntries = [];
            }
            try {
                wizard.hostsEntries = JSON.parse(backend.hosts_status_json);
            } catch (e) {
                wizard.hostsEntries = [];
            }
        }
    }

    FolderDialog {
        id: gamesDirDialog
        title: "Select games directory"
        onAccepted: backend.set_games_dir(selectedFolder.toString().replace("file://", ""))
    }

    StackLayout {
        anchors.fill: parent
        currentIndex: backend.current_page

        ColumnLayout {
            spacing: 10
            Label {
                text: "Welcome to Retouched!"
                font.bold: true
                font.pixelSize: 18
            }
            Item {
                Layout.preferredHeight: 10
            }
            Label {
                text: "This wizard will help you set up your environment."
            }
            Label {
                text: "Each step can be skipped and configured later from Settings."
            }
            Item {
                Layout.fillHeight: true
            }
            Button {
                text: "Next >"
                onClicked: backend.next_page()
            }
        }

        ColumnLayout {
            spacing: 8
            Label {
                text: "Flash Player Trust"
                font.bold: true
                font.pixelSize: 16
            }

            Repeater {
                model: wizard.trustEntries
                Label {
                    text: "  " + modelData
                    color: "#00c800"
                }
            }

            RowLayout {
                spacing: 8
                Label {
                    text: "Games directory:"
                }
                TextField {
                    text: backend.games_directory
                    Layout.fillWidth: true
                    onEditingFinished: backend.set_games_dir(text)
                }
                Button {
                    text: "Browse..."
                    onClicked: gamesDirDialog.open()
                }
            }

            Button {
                text: "Write trust config"
                visible: backend.games_directory !== "" && !backend.trust_written
                onClicked: backend.write_trust_config()
            }

            Label {
                text: "Trust config written"
                color: "#00c800"
                visible: backend.trust_written
            }

            Item {
                Layout.fillHeight: true
            }
            RowLayout {
                spacing: 8
                Button {
                    text: "< Back"
                    onClicked: backend.prev_page()
                }
                Button {
                    text: "Next >"
                    onClicked: backend.next_page()
                }
            }
        }

        ColumnLayout {
            spacing: 8
            Label {
                text: "Hosts File Redirect"
                font.bold: true
                font.pixelSize: 16
            }

            Label {
                text: "Hosts entries already configured"
                color: "#00c800"
                visible: backend.hosts_already_configured
            }

            Repeater {
                model: wizard.hostsEntries
                Label {
                    text: (modelData.status === "ok" ? "[OK] " : "[--] ") + modelData.domain + (modelData.status === "ok" ? " -> " + modelData.ip : " -- not configured")
                }
            }

            RowLayout {
                visible: !backend.hosts_already_configured
                spacing: 8
                Label {
                    text: "Redirect IP:"
                }
                TextField {
                    text: backend.hosts_redirect_ip
                    implicitWidth: 200
                    onEditingFinished: backend.set_hosts_ip_value(text)
                }
            }

            Button {
                text: "Apply hosts redirect"
                visible: !backend.hosts_already_configured
                onClicked: backend.apply_hosts()
            }

            Item {
                Layout.fillHeight: true
            }
            RowLayout {
                spacing: 8
                Button {
                    text: "< Back"
                    onClicked: backend.prev_page()
                }
                Button {
                    text: "Next >"
                    onClicked: backend.next_page()
                }
            }
        }

        ColumnLayout {
            spacing: 8
            Label {
                text: "Firewall Ports"
                font.bold: true
                font.pixelSize: 16
            }

            Label {
                text: "Required ports:\nTCP 8080 (HTTP Server)\nTCP 8088 (BM Registry)\nTCP 8089 (Retouched Web)\nTCP 8443 (WebRTC Bridge)\nTCP 9081 (Game)"
                opacity: 0.7
            }

            Label {
                text: "Detected backend: " + backend.firewall_backend_name
            }

            Label {
                text: "No supported firewall manager detected.\nPlease open the ports manually."
                color: "#ffc800"
                visible: backend.firewall_backend_name === "None"
            }

            Button {
                text: "Open ports"
                visible: backend.firewall_backend_name !== "None" && backend.firewall_backend_name !== "" && !backend.firewall_opened
                onClicked: backend.open_firewall_ports()
            }

            Label {
                text: "Ports opened"
                color: "#00c800"
                visible: backend.firewall_opened
            }

            Item {
                Layout.fillHeight: true
            }
            RowLayout {
                spacing: 8
                Button {
                    text: "< Back"
                    onClicked: backend.prev_page()
                }
                Button {
                    text: "Next >"
                    onClicked: backend.next_page()
                }
            }
        }

        ColumnLayout {
            spacing: 10
            Label {
                text: "Setup Complete"
                font.bold: true
                font.pixelSize: 18
            }
            Item {
                Layout.preferredHeight: 10
            }
            Label {
                text: "You can change any of these settings later from the Settings tab."
            }
            Item {
                Layout.fillHeight: true
            }
            Button {
                text: "Done"
                onClicked: backend.finish()
            }
        }
    }
}
