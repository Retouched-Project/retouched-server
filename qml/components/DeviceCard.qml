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

    implicitWidth: contentRow.implicitWidth + leftPadding + rightPadding

    background: Rectangle {
        color: "transparent"
        border.color: root.flashing ? "#00ff64" : (root.isConnected ? Qt.darker(root.typeColor, 1.5) : root.palette.mid)
        border.width: root.flashing ? 2 : 1
        radius: 4
    }

    RowLayout {
        id: contentRow
        anchors.fill: parent
        spacing: 8

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
    }
}
