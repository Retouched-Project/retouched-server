// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 ddavef/KinteLiX retouched-server

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: logViewer

    property bool logVisible: false
    property bool autoScroll: true
    property int levelFilter: 3
    property var entries: []

    implicitHeight: logVisible ? 250 : 0
    clip: true

    ListView {
        id: logList
        anchors.fill: parent
        model: logViewer.entries
        clip: true

        delegate: Text {
            width: logList.width
            text: "[" + modelData.level + "] " + modelData.message
            color: modelData.color
            font.family: "monospace"
            font.pixelSize: 12
            wrapMode: Text.NoWrap
            elide: Text.ElideRight
        }

        onCountChanged: {
            if (logViewer.autoScroll) {
                logList.positionViewAtEnd();
            }
        }

        ScrollBar.vertical: ScrollBar {
            policy: ScrollBar.AsNeeded
        }
    }

    Label {
        anchors.centerIn: parent
        text: "No log entries"
        visible: logViewer.entries.length === 0 && logViewer.logVisible
        opacity: 0.5
    }
}
