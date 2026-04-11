// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 ddavef/KinteLiX retouched-server

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Rectangle {
    id: titleBar

    required property ApplicationWindow window

    height: 34
    color: titleBar.window.active ? Qt.darker(palette.window, 1.15) : palette.window
    topLeftRadius: titleBar.window.cornerRadius
    topRightRadius: titleBar.window.cornerRadius

    DragHandler {
        target: null
        onActiveChanged: if (active)
            titleBar.window.startSystemMove()
    }

    TapHandler {
        onDoubleTapped: {
            if (titleBar.window.visibility === Window.Maximized)
                titleBar.window.showNormal();
            else
                titleBar.window.showMaximized();
        }
    }

    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: 10
        spacing: 0

        Image {
            source: "qrc:/assets/retouched_logo_icons.png"
            sourceSize: Qt.size(18, 18)
            Layout.alignment: Qt.AlignVCenter
        }

        Label {
            text: titleBar.window.title
            font.pixelSize: 13
            Layout.leftMargin: 8
            Layout.alignment: Qt.AlignVCenter
        }

        Item {
            Layout.fillWidth: true
        }

        AbstractButton {
            id: minimizeBtn
            Layout.preferredWidth: 46
            Layout.preferredHeight: titleBar.height

            background: Rectangle {
                color: minimizeBtn.hovered ? Qt.lighter(palette.window, 1.4) : "transparent"
            }

            contentItem: Item {
                Rectangle {
                    anchors.centerIn: parent
                    width: 10
                    height: 1
                    color: palette.windowText
                }
            }

            onClicked: titleBar.window.showMinimized()
        }

        AbstractButton {
            id: maximizeBtn
            Layout.preferredWidth: 46
            Layout.preferredHeight: titleBar.height

            property bool isMaximized: titleBar.window.visibility === Window.Maximized

            background: Rectangle {
                color: maximizeBtn.hovered ? Qt.lighter(palette.window, 1.4) : "transparent"
            }

            contentItem: Item {
                Item {
                    anchors.centerIn: parent
                    width: 10
                    height: 10
                    visible: !maximizeBtn.isMaximized

                    Rectangle {
                        anchors.fill: parent
                        color: "transparent"
                        border.color: palette.windowText
                        border.width: 1
                    }
                }

                Item {
                    anchors.centerIn: parent
                    width: 12
                    height: 12
                    visible: maximizeBtn.isMaximized

                    Rectangle {
                        x: 3
                        y: 0
                        width: 9
                        height: 9
                        color: "transparent"
                        border.color: palette.windowText
                        border.width: 1
                    }
                    Rectangle {
                        x: 0
                        y: 3
                        width: 9
                        height: 9
                        color: maximizeBtn.hovered ? Qt.lighter(palette.window, 1.4) : Qt.darker(palette.window, 1.15)
                        border.color: palette.windowText
                        border.width: 1
                    }
                }
            }

            onClicked: {
                if (isMaximized)
                    titleBar.window.showNormal();
                else
                    titleBar.window.showMaximized();
            }
        }

        AbstractButton {
            id: closeBtn
            Layout.preferredWidth: 46
            Layout.preferredHeight: titleBar.height

            background: Rectangle {
                color: closeBtn.hovered ? "#e81123" : "transparent"
                topRightRadius: titleBar.topRightRadius
            }

            contentItem: Item {
                Rectangle {
                    anchors.centerIn: parent
                    width: 14
                    height: 1
                    rotation: 45
                    color: closeBtn.hovered ? "#ffffff" : palette.windowText
                    antialiasing: true
                }
                Rectangle {
                    anchors.centerIn: parent
                    width: 14
                    height: 1
                    rotation: -45
                    color: closeBtn.hovered ? "#ffffff" : palette.windowText
                    antialiasing: true
                }
            }

            onClicked: titleBar.window.close()
        }
    }

    Rectangle {
        anchors.bottom: parent.bottom
        anchors.left: parent.left
        anchors.right: parent.right
        height: 1
        color: Qt.darker(palette.window, 1.4)
    }
}
