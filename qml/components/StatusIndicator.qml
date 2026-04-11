// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 ddavef/KinteLiX retouched-server

import QtQuick

Rectangle {
    property string status: "stopped"

    width: 12
    height: 12
    radius: 6

    color: {
        switch (status) {
        case "running":
            return "#00c800";
        case "starting":
        case "stopping":
            return "#c8c800";
        case "stopped":
            return "#c80000";
        case "error":
            return "#ff5050";
        default:
            return "#b4b4b4";
        }
    }
}
