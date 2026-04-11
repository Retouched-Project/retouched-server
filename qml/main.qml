// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 ddavef/KinteLiX retouched-server

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import com.retouched.server

ApplicationWindow {
    id: root
    visible: true
    width: 900
    height: 650
    minimumWidth: 600
    minimumHeight: 400
    title: "Retouched Server"
    flags: useCsd ? (Qt.FramelessWindowHint | Qt.Window) : Qt.Window
    color: useCsd ? "transparent" : palette.window

    property int cornerRadius: useCsd && root.visibility !== Window.Maximized ? 8 : 0

    background: Rectangle {
        radius: root.cornerRadius
        color: palette.window
    }

    onClosing: function (close) {
        close.accepted = false;
        root.hide();
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        TitleBar {
            window: root
            Layout.fillWidth: true
            visible: useCsd
        }

        TabBar {
            id: tabBar
            Layout.fillWidth: true
            TabButton {
                text: "Server"
            }
            TabButton {
                text: "Retouched Web"
            }
            TabButton {
                text: "Touchy Patcher"
            }
            TabButton {
                text: "Settings"
            }
        }

        StackLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            currentIndex: tabBar.currentIndex

            ServerTab {}
            WebAppTab {}
            PatcherTab {}
            SettingsTab {}
        }
    }

    SetupWizard {}

    Item {
        anchors.fill: parent
        z: 1000
        visible: useCsd && root.visibility !== Window.Maximized

        MouseArea {
            anchors.top: parent.top
            anchors.left: parent.left
            width: 8
            height: 8
            cursorShape: Qt.SizeFDiagCursor
            onPressed: root.startSystemResize(Qt.TopEdge | Qt.LeftEdge)
        }
        MouseArea {
            anchors.top: parent.top
            anchors.right: parent.right
            width: 8
            height: 8
            cursorShape: Qt.SizeBDiagCursor
            onPressed: root.startSystemResize(Qt.TopEdge | Qt.RightEdge)
        }
        MouseArea {
            anchors.bottom: parent.bottom
            anchors.left: parent.left
            width: 8
            height: 8
            cursorShape: Qt.SizeBDiagCursor
            onPressed: root.startSystemResize(Qt.BottomEdge | Qt.LeftEdge)
        }
        MouseArea {
            anchors.bottom: parent.bottom
            anchors.right: parent.right
            width: 8
            height: 8
            cursorShape: Qt.SizeFDiagCursor
            onPressed: root.startSystemResize(Qt.BottomEdge | Qt.RightEdge)
        }

        MouseArea {
            anchors.top: parent.top
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.leftMargin: 8
            anchors.rightMargin: 8
            height: 5
            cursorShape: Qt.SizeVerCursor
            onPressed: root.startSystemResize(Qt.TopEdge)
        }
        MouseArea {
            anchors.bottom: parent.bottom
            anchors.left: parent.left
            anchors.right: parent.right
            anchors.leftMargin: 8
            anchors.rightMargin: 8
            height: 5
            cursorShape: Qt.SizeVerCursor
            onPressed: root.startSystemResize(Qt.BottomEdge)
        }
        MouseArea {
            anchors.top: parent.top
            anchors.bottom: parent.bottom
            anchors.left: parent.left
            anchors.topMargin: 8
            anchors.bottomMargin: 8
            width: 5
            cursorShape: Qt.SizeHorCursor
            onPressed: root.startSystemResize(Qt.LeftEdge)
        }
        MouseArea {
            anchors.top: parent.top
            anchors.bottom: parent.bottom
            anchors.right: parent.right
            anchors.topMargin: 8
            anchors.bottomMargin: 8
            width: 5
            cursorShape: Qt.SizeHorCursor
            onPressed: root.startSystemResize(Qt.RightEdge)
        }
    }
}
