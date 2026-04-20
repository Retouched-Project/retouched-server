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

    onClosing: function (close) {
        close.accepted = false;
        root.hide();
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
}
