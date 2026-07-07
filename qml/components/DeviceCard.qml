// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 ddavef/KinteLiX retouched-server

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Frame {
    id: root

    property string deviceName: ""
    property string deviceType: ""
    property string appLabel: ""
    property string statusText: ""
    property bool isConnected: false
    property string connectionTime: ""
    property color typeColor: "#b4b4b4"
    property bool flashing: false
    property bool isRetouched: false
    property string iconUrl: ""
    property int slotId: 0
    property color slotColor: "#666666"
    property int currentPlayers: 0
    property int maxPlayers: 0

    // registry detail, revealed on expand
    property bool expanded: false
    property string deviceId: ""
    property string appId: ""
    property string addr: ""
    property string address: ""
    property int reliablePort: 0
    property int unreliablePort: 0
    property string domain: ""

    signal toggleExpand()

    implicitWidth: (root.expanded ? Math.max(contentRow.implicitWidth, 320) : contentRow.implicitWidth) + leftPadding + rightPadding

    background: Rectangle {
        color: "transparent"
        border.color: root.flashing ? "#00ff64" : (root.isConnected ? Qt.darker(root.typeColor, 1.5) : root.palette.mid)
        border.width: root.flashing ? 2 : 1
        radius: 4
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 6

        RowLayout {
            id: contentRow
            Layout.fillWidth: true
            spacing: 8

            TapHandler {
                onTapped: root.toggleExpand()
            }

            Image {
                id: appIcon
                source: root.iconUrl
                sourceSize.width: 40
                sourceSize.height: 40
                Layout.preferredWidth: 40
                Layout.preferredHeight: 40
                Layout.alignment: Qt.AlignVCenter
                fillMode: Image.PreserveAspectFit
                visible: root.iconUrl !== "" && status === Image.Ready
            }

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 2

                Label {
                    text: root.deviceName
                    font.bold: true
                    color: root.typeColor
                }

                RowLayout {
                    spacing: 4
                    Image {
                        source: "qrc:/assets/retouched_logo.svg"
                        sourceSize.width: 14
                        sourceSize.height: 14
                        visible: root.isRetouched
                        Layout.alignment: Qt.AlignVCenter
                    }
                    Label {
                        text: root.appLabel
                        font.pixelSize: 11
                    }
                    Label {
                        text: root.deviceType
                        font.pixelSize: 10
                        opacity: 0.6
                    }
                }

                Label {
                    text: root.statusText
                    font.pixelSize: 10
                    color: root.isConnected ? "#00c864" : "#646464"
                    elide: Text.ElideRight
                    Layout.fillWidth: true
                }
            }

            ColumnLayout {
                spacing: 4
                Layout.alignment: Qt.AlignRight | Qt.AlignTop

                RowLayout {
                    spacing: 6
                    Layout.alignment: Qt.AlignRight
                    visible: root.slotId > 0

                    Item {
                        width: 28
                        height: 28
                        Rectangle {
                            anchors.fill: parent
                            radius: 4
                            color: root.slotColor
                        }
                        Image {
                            anchors.fill: parent
                            source: "qrc:/assets/slotwifi.svg"
                            sourceSize.width: 28
                            sourceSize.height: 28
                            fillMode: Image.PreserveAspectFit
                        }
                    }

                    Label {
                        text: root.currentPlayers + "/" + root.maxPlayers
                        font.pixelSize: 11
                        opacity: 0.8
                    }
                }

                Item {
                    Layout.fillHeight: true
                }

                Label {
                    text: root.connectionTime
                    font.pixelSize: 10
                    opacity: 0.5
                    Layout.alignment: Qt.AlignRight
                }
            }

            Label {
                text: "▸"
                font.pixelSize: 12
                opacity: 0.6
                rotation: root.expanded ? 90 : 0
                Layout.alignment: Qt.AlignVCenter
                Behavior on rotation {
                    NumberAnimation {
                        duration: 120
                    }
                }
            }
        }

        Rectangle {
            visible: root.expanded
            Layout.fillWidth: true
            height: 1
            color: root.palette.mid
            opacity: 0.5
        }

        GridLayout {
            visible: root.expanded
            Layout.fillWidth: true
            columns: 3
            columnSpacing: 8
            rowSpacing: 3

            component Key: Label {
                font.pixelSize: 10
                opacity: 0.6
                Layout.alignment: Qt.AlignTop
            }
            component Val: Label {
                font.pixelSize: 10
                font.family: "monospace"
                wrapMode: Text.WrapAnywhere
                Layout.fillWidth: true
            }

            Key {
                text: "App ID"
            }
            Val {
                text: root.appId
            }
            CopyButton {
                text: root.appId
                Layout.alignment: Qt.AlignTop
            }

            Key {
                text: "Device ID"
            }
            Val {
                text: root.deviceId
            }
            CopyButton {
                text: root.deviceId
                Layout.alignment: Qt.AlignTop
            }

            Key {
                text: "Slot"
            }
            Val {
                text: root.slotId.toString()
            }
            CopyButton {
                text: root.slotId.toString()
                Layout.alignment: Qt.AlignTop
            }

            Key {
                text: "Address"
            }
            Val {
                text: root.address
            }
            CopyButton {
                text: root.address
                Layout.alignment: Qt.AlignTop
            }

            Key {
                text: "Reliable port"
            }
            Val {
                text: root.reliablePort.toString()
            }
            CopyButton {
                text: root.reliablePort.toString()
                Layout.alignment: Qt.AlignTop
            }

            Key {
                text: "Unreliable port"
            }
            Val {
                text: root.unreliablePort.toString()
            }
            CopyButton {
                text: root.unreliablePort.toString()
                Layout.alignment: Qt.AlignTop
            }

            Key {
                text: "Peer"
            }
            Val {
                text: root.addr
            }
            CopyButton {
                text: root.addr
                Layout.alignment: Qt.AlignTop
            }

            Key {
                text: "Domain"
                visible: root.domain !== ""
            }
            Val {
                text: root.domain
                visible: root.domain !== ""
            }
            CopyButton {
                text: root.domain
                visible: root.domain !== ""
                Layout.alignment: Qt.AlignTop
            }
        }
    }
}
