// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 ddavef/KinteLiX retouched-server

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import com.retouched.server

ApplicationWindow {
    id: root
    visible: false
    width: 900
    height: 650
    minimumWidth: 600
    minimumHeight: 400
    title: "Retouched Server"

    // shared log state, so the panel looks the same on every tab
    property bool logExpanded: false
    property bool logAutoScroll: true
    property int logLevelFilter: 3
    property var logData: []
    // the log docks only on these tabs: Server, Retouched Web, Touchy Patcher
    property bool logAvailable: tabBar.currentIndex < 3
    property real logHeight: 250

    onClosing: function (close) {
        windowBackend.save_size(root.width, root.height);
        close.accepted = false;
        root.hide();
    }

    // restore the saved size before showing, so there is no resize flash
    Component.onCompleted: {
        var w = windowBackend.saved_width();
        var h = windowBackend.saved_height();
        if (w >= root.minimumWidth && h >= root.minimumHeight) {
            root.width = w;
            root.height = h;
        }
        root.visible = true;
    }

    onWidthChanged: saveSizeTimer.restart()
    onHeightChanged: saveSizeTimer.restart()

    LogBackend {
        id: logBackend
    }

    WindowBackend {
        id: windowBackend
    }

    Timer {
        id: saveSizeTimer
        interval: 500
        onTriggered: windowBackend.save_size(root.width, root.height)
    }

    Timer {
        interval: 200
        running: root.logExpanded && root.logAvailable
        repeat: true
        onTriggered: {
            try {
                root.logData = JSON.parse(logBackend.log_entries_json(root.logLevelFilter));
            } catch (e) {
                root.logData = [];
            }
        }
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

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
            TabButton {
                text: "About"
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
            AboutTab {}
        }

        Rectangle {
            Layout.fillWidth: true
            height: root.logExpanded ? 6 : 1
            color: (root.logExpanded && (resizeArea.containsMouse || resizeArea.pressed)) ? palette.highlight : palette.mid
            visible: root.logAvailable

            MouseArea {
                id: resizeArea
                anchors.fill: parent
                enabled: root.logExpanded
                hoverEnabled: true
                cursorShape: Qt.SizeVerCursor
                property real lastY: 0
                onPressed: mouse => resizeArea.lastY = resizeArea.mapToItem(null, mouse.x, mouse.y).y
                onPositionChanged: mouse => {
                    if (!resizeArea.pressed) {
                        return;
                    }
                    var y = resizeArea.mapToItem(null, mouse.x, mouse.y).y;
                    root.logHeight = Math.max(120, Math.min(root.height * 0.7, root.logHeight - (y - resizeArea.lastY)));
                    resizeArea.lastY = y;
                }
            }
        }

        RowLayout {
            Layout.fillWidth: true
            Layout.leftMargin: 8
            Layout.rightMargin: 8
            spacing: 8
            visible: root.logAvailable

            Canvas {
                width: 10
                height: 10
                Layout.alignment: Qt.AlignVCenter
                rotation: root.logExpanded ? 90 : 0
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
                MouseArea {
                    anchors.fill: parent
                    onClicked: root.logExpanded = !root.logExpanded
                }
            }

            Button {
                text: root.logExpanded ? "Hide Log" : "Show Log"
                flat: true
                onClicked: root.logExpanded = !root.logExpanded
            }

            Item {
                Layout.fillWidth: true
            }

            Button {
                text: "Clear"
                visible: root.logExpanded
                onClicked: logBackend.clear_log()
            }

            ComboBox {
                visible: root.logExpanded
                model: ["Error", "Warn", "Info", "Debug", "Trace"]
                currentIndex: root.logLevelFilter - 1
                onCurrentIndexChanged: root.logLevelFilter = currentIndex + 1
            }

            CheckBox {
                visible: root.logExpanded
                text: "Auto-scroll"
                checked: root.logAutoScroll
                onCheckedChanged: root.logAutoScroll = checked
            }
        }

        LogViewer {
            Layout.fillWidth: true
            Layout.preferredHeight: (root.logExpanded && root.logAvailable) ? root.logHeight : 0
            visible: root.logExpanded && root.logAvailable
            logVisible: root.logExpanded
            autoScroll: root.logAutoScroll
            levelFilter: root.logLevelFilter
            entries: root.logData
        }
    }

    SetupWizard {}
}
