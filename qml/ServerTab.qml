// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 ddavef/KinteLiX retouched-server

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import com.retouched.server

Item {
    id: serverTab

    property var expandedCards: ({})

    function toggleCard(deviceId) {
        var m = Object.assign({}, serverTab.expandedCards);
        m[deviceId] = !m[deviceId];
        serverTab.expandedCards = m;
    }

    function syncModel(model, items) {
        var present = {};
        for (var a = 0; a < items.length; a++)
            present[items[a].deviceId] = true;
        for (var r = model.count - 1; r >= 0; r--) {
            if (!present[model.get(r).deviceId])
                model.remove(r);
        }
        for (var i = 0; i < items.length; i++) {
            var it = items[i];
            var idx = -1;
            for (var j = 0; j < model.count; j++) {
                if (model.get(j).deviceId === it.deviceId) {
                    idx = j;
                    break;
                }
            }
            if (idx === -1)
                model.append(it);
            else
                model.set(idx, it);
        }
    }

    ListModel {
        id: gamesModel
    }

    ListModel {
        id: controllersModel
    }

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
                var data = JSON.parse(backend.client_data_json());
                serverTab.syncModel(gamesModel, data.games || []);
                serverTab.syncModel(controllersModel, data.controllers || []);
            } catch (e) {
                gamesModel.clear();
                controllersModel.clear();
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

        ScrollView {
            id: clientsScroll
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            ScrollBar.horizontal.policy: ScrollBar.AlwaysOff

            ColumnLayout {
                width: clientsScroll.availableWidth
                spacing: 6

                Label {
                    text: "Games (" + gamesModel.count + ")"
                    font.bold: true
                    font.pixelSize: 16
                }

                Flow {
                    Layout.fillWidth: true
                    spacing: 6

                    Repeater {
                        model: gamesModel
                        DeviceCard {
                            expanded: serverTab.expandedCards[model.deviceId] === true
                            onToggleExpand: serverTab.toggleCard(model.deviceId)
                            deviceName: model.name
                            deviceType: model.typeName
                            appLabel: model.appLabel
                            statusText: model.status
                            isConnected: model.controllerCount > 0
                            connectionTime: model.connectionTime
                            typeColor: model.typeColor
                            flashing: model.flashing
                            isRetouched: model.isRetouched
                            iconUrl: model.iconUrl
                            slotId: model.slotId
                            slotColor: model.slotColor
                            currentPlayers: model.currentPlayers
                            maxPlayers: model.maxPlayers
                            deviceId: model.deviceId
                            appId: model.appId
                            addr: model.addr
                            address: model.address
                            reliablePort: model.reliablePort
                            unreliablePort: model.unreliablePort
                            domain: model.domain
                        }
                    }
                }

                Label {
                    text: "No games connected."
                    visible: gamesModel.count === 0
                    opacity: 0.5
                }

                Label {
                    text: "Controllers (" + controllersModel.count + ")"
                    font.bold: true
                    font.pixelSize: 16
                }

                Flow {
                    Layout.fillWidth: true
                    spacing: 6

                    Repeater {
                        model: controllersModel
                        DeviceCard {
                            expanded: serverTab.expandedCards[model.deviceId] === true
                            onToggleExpand: serverTab.toggleCard(model.deviceId)
                            deviceName: model.name
                            deviceType: model.typeName
                            appLabel: model.appLabel
                            statusText: model.connectedGame !== "" ? model.connectedGame : "Idle"
                            isConnected: model.connectedGame !== ""
                            connectionTime: model.connectionTime
                            typeColor: model.typeColor
                            flashing: model.flashing
                            isRetouched: model.isRetouched
                            slotId: model.slotId
                            deviceId: model.deviceId
                            appId: model.appId
                            addr: model.addr
                            address: model.address
                            reliablePort: model.reliablePort
                            unreliablePort: model.unreliablePort
                            domain: model.domain
                        }
                    }
                }

                Label {
                    text: "No controllers connected."
                    visible: controllersModel.count === 0
                    opacity: 0.5
                }
            }
        }
    }
}
