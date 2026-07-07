// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 ddavef/KinteLiX retouched-server

import QtQuick
import QtQuick.Controls

Item {
    id: cb

    property string text: ""
    property bool copied: false

    implicitWidth: 16
    implicitHeight: 16

    Canvas {
        id: canvas
        anchors.fill: parent

        onPaint: {
            var ctx = getContext("2d");
            ctx.reset();

            if (cb.copied) {
                ctx.strokeStyle = "#4caf50";
                ctx.lineWidth = 1.5;
                ctx.beginPath();
                ctx.moveTo(3, 8);
                ctx.lineTo(6.5, 12);
                ctx.lineTo(13, 4);
                ctx.stroke();
                return;
            }

            var hot = ma.containsMouse;
            ctx.lineWidth = 1;
            ctx.strokeStyle = hot ? cb.palette.highlight : cb.palette.text;
            ctx.globalAlpha = hot ? 0.9 : 0.4;
            ctx.strokeRect(5.5, 2.5, 7, 9);
            ctx.globalAlpha = 1.0;
            ctx.fillStyle = cb.palette.base;
            ctx.fillRect(2.5, 5.5, 7, 9);
            ctx.globalAlpha = hot ? 0.9 : 0.4;
            ctx.strokeRect(2.5, 5.5, 7, 9);
        }
    }

    MouseArea {
        id: ma
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        onContainsMouseChanged: canvas.requestPaint()
        onClicked: {
            if (cb.text === "") {
                return;
            }
            buf.text = cb.text;
            buf.selectAll();
            buf.copy();
            cb.copied = true;
            canvas.requestPaint();
            resetTimer.restart();
        }
    }

    Timer {
        id: resetTimer
        interval: 900
        onTriggered: {
            cb.copied = false;
            canvas.requestPaint();
        }
    }

    // plain-text clipboard buffer, so no rich markup is ever copied
    TextEdit {
        id: buf
        visible: false
        textFormat: TextEdit.PlainText
    }

    ToolTip.visible: ma.containsMouse
    ToolTip.text: cb.copied ? "Copied" : "Copy"
}
