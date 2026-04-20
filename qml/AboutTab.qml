// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 ddavef/KinteLiX retouched-server

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: aboutTab

    ColumnLayout {
        anchors.centerIn: parent
        width: Math.min(parent.width - 40, 560)
        spacing: 20

        Image {
            Layout.alignment: Qt.AlignHCenter
            source: "qrc:/assets/retouched_logo_text_server.svg"
            sourceSize.width: 360
            fillMode: Image.PreserveAspectFit
            smooth: true
        }

        Label {
            Layout.alignment: Qt.AlignHCenter
            text: "Version " + Qt.application.version
            opacity: 0.8
        }

        Label {
            Layout.alignment: Qt.AlignHCenter
            text: "Copyright © 2026 ddavef/KinteLiX"
        }

        Label {
            Layout.alignment: Qt.AlignHCenter
            Layout.fillWidth: true
            horizontalAlignment: Text.AlignHCenter
            wrapMode: Text.WordWrap
            text: "Licensed under the GNU Affero General Public License v3.0 or later.\n" +
                  "This program is free software: you can redistribute it and/or modify it under " +
                  "the terms of the AGPL as published by the Free Software Foundation."
            opacity: 0.85
        }

        RowLayout {
            Layout.alignment: Qt.AlignHCenter
            spacing: 12

            Button {
                text: "Organization"
                onClicked: Qt.openUrlExternally("https://github.com/Retouched-Project")
            }

            Button {
                text: "Repository"
                onClicked: Qt.openUrlExternally("https://github.com/Retouched-Project/retouched-server")
            }
        }
    }
}
