// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 ddavef/KinteLiX retouched-server

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import com.retouched.server

Item {
    id: serverTab

    property bool logVisible: false
    property bool logAutoScroll: true
    property int logLevelFilter: 3
    property var clientData: ({
            games: [],
            controllers: []
        })
    property var logData: []

    ServerBackend {
        id: backend
    }

    Timer {
        interval: 200
        running: true
        repeat: true
        onTriggered: {
            backend.refresh();
            try {
                serverTab.clientData = JSON.parse(backend.client_data_json());
            } catch (e) {
                serverTab.clientData = {
                    games: [],
                    controllers: []
                };
            }
            if (serverTab.logVisible) {
                try {
                    serverTab.logData = JSON.parse(backend.log_entries_json(serverTab.logLevelFilter));
                } catch (e) {
                    serverTab.logData = [];
                }
            }
        }
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 8
        spacing: 6

        RowLayout {
            spacing: 8

            Button {
                text: {
                    switch (backend.server_status) {
                    case "Running":
                        return "Stop Server";
                    case "Starting":
                        return "Starting...";
                    case "Stopping":
                        return "Stopping...";
                    default:
                        return "Start Server";
                    }
                }
                enabled: backend.server_status === "Stopped" || backend.server_status === "Running"
                onClicked: {
                    if (backend.server_status === "Stopped")
                        backend.start_server();
                    else if (backend.server_status === "Running")
                        backend.stop_server();
                }
            }

            StatusIndicator {
                status: backend.server_status.toLowerCase()
                Layout.alignment: Qt.AlignVCenter
            }

            Label {
                text: backend.server_status
                color: {
                    switch (backend.server_status) {
                    case "Running":
                        return "#00c800";
                    case "Stopped":
                        return "#c80000";
                    default:
                        return "#c8c800";
                    }
                }
            }

            Label {
                text: "Uptime: " + backend.uptime
                visible: backend.server_status === "Running"
                opacity: 0.7
            }

            Item {
                Layout.fillWidth: true
            }

            Label {
                text: backend.lan_ip
                visible: backend.server_status === "Running" && backend.lan_ip !== ""
                opacity: 0.5
                font.family: "monospace"
            }
        }

        Rectangle {
            Layout.fillWidth: true
            height: 1
            color: palette.mid
        }

        Label {
            text: "Games (" + serverTab.clientData.games.length + ")"
            font.bold: true
            font.pixelSize: 16
        }

        Flow {
            Layout.fillWidth: true
            spacing: 6

            Repeater {
                model: serverTab.clientData.games
                DeviceCard {
                    deviceName: modelData.name
                    deviceType: modelData.typeName
                    appLabel: modelData.appLabel
                    statusText: modelData.controllerNames.length > 0 ? modelData.controllerNames.join(", ") : "Idle"
                    isConnected: modelData.controllerCount > 0
                    connectionTime: modelData.connectionTime
                    typeColor: modelData.typeColor
                    flashing: modelData.flashing
                    isRetouched: modelData.isRetouched || false
                    iconUrl: modelData.iconUrl || ""
                    slotId: modelData.slotId || 0
                    slotColor: modelData.slotColor || "#666666"
                    currentPlayers: modelData.currentPlayers || 0
                    maxPlayers: modelData.maxPlayers || 0
                }
            }
        }

        Label {
            text: "No games connected."
            visible: serverTab.clientData.games.length === 0
            opacity: 0.5
        }

        Label {
            text: "Controllers (" + serverTab.clientData.controllers.length + ")"
            font.bold: true
            font.pixelSize: 16
        }

        Flow {
            Layout.fillWidth: true
            spacing: 6

            Repeater {
                model: serverTab.clientData.controllers
                DeviceCard {
                    deviceName: modelData.name
                    deviceType: modelData.typeName
                    appLabel: modelData.appLabel
                    statusText: modelData.connectedGame ? (modelData.connectedGame) : "Idle"
                    isConnected: !!modelData.connectedGame
                    connectionTime: modelData.connectionTime
                    typeColor: modelData.typeColor
                    flashing: modelData.flashing
                    isRetouched: modelData.isRetouched || false
                }
            }
        }

        Label {
            text: "No controllers connected."
            visible: serverTab.clientData.controllers.length === 0
            opacity: 0.5
        }

        Item {
            Layout.fillHeight: true
        }

        Rectangle {
            Layout.fillWidth: true
            height: 1
            color: palette.mid
        }

        RowLayout {
            spacing: 8

            Canvas {
                width: 10
                height: 10
                Layout.alignment: Qt.AlignVCenter
                rotation: serverTab.logVisible ? 90 : 0
                Behavior on rotation {
                    NumberAnimation {
                        duration: 150
                    }
                }
                onPaint: {
                    var ctx = getContext("2d");
                    ctx.reset();
                    ctx.fillStyle = palette.text;
                    ctx.beginPath();
                    ctx.moveTo(2, 1);
                    ctx.lineTo(8, 5);
                    ctx.lineTo(2, 9);
                    ctx.closePath();
                    ctx.fill();
                }
            }

            Button {
                text: "Log"
                flat: true
                onClicked: serverTab.logVisible = !serverTab.logVisible
            }

            Item {
                visible: serverTab.logVisible
                Layout.fillWidth: true
            }

            Button {
                text: "Clear"
                visible: serverTab.logVisible
                onClicked: backend.clear_log()
            }

            ComboBox {
                visible: serverTab.logVisible
                model: ["Error", "Warn", "Info", "Debug", "Trace"]
                currentIndex: serverTab.logLevelFilter - 1
                onCurrentIndexChanged: serverTab.logLevelFilter = currentIndex + 1
            }

            CheckBox {
                visible: serverTab.logVisible
                text: "Auto-scroll"
                checked: serverTab.logAutoScroll
                onCheckedChanged: serverTab.logAutoScroll = checked
            }
        }

        LogViewer {
            Layout.fillWidth: true
            Layout.fillHeight: true
            visible: serverTab.logVisible
            logVisible: serverTab.logVisible
            autoScroll: serverTab.logAutoScroll
            levelFilter: serverTab.logLevelFilter
            entries: serverTab.logData
        }
    }
}
