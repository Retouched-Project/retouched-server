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

    property string _sig: ""

    implicitHeight: logVisible ? 250 : 0
    clip: true

    function escapeHtml(s) {
        return String(s).replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
    }

    function rebuild() {
        var e = logViewer.entries;
        var sig = e.length + "|" + (e.length ? e[e.length - 1].time : "");
        if (sig === logViewer._sig) {
            return;
        }
        logViewer._sig = sig;

        var hadSelection = edit.selectionStart !== edit.selectionEnd;
        var selStart = edit.selectionStart;
        var selEnd = edit.selectionEnd;

        var parts = [];
        for (var i = 0; i < e.length; i++) {
            parts.push('[<span style="color:' + e[i].color + '">' + e[i].level + '</span>] ' + escapeHtml(e[i].message));
        }
        edit.text = parts.join('<br>');

        // keep an existing selection in place, only follow the tail when nothing is selected
        if (hadSelection) {
            edit.select(selStart, selEnd);
        } else if (logViewer.autoScroll) {
            Qt.callLater(scrollToEnd);
        }
    }

    function scrollToEnd() {
        flick.contentY = Math.max(0, flick.contentHeight - flick.height);
    }

    onEntriesChanged: rebuild()

    Flickable {
        id: flick
        anchors.fill: parent
        contentWidth: width
        contentHeight: edit.implicitHeight
        clip: true
        boundsBehavior: Flickable.StopAtBounds

        ScrollBar.vertical: ScrollBar {
            policy: ScrollBar.AsNeeded
        }

        TextEdit {
            id: edit
            width: flick.width
            readOnly: true
            selectByMouse: true
            persistentSelection: true
            textFormat: TextEdit.RichText
            wrapMode: TextEdit.Wrap
            font.family: "monospace"
            font.pixelSize: 12
            color: palette.text
            text: ""

            // copy plain text only, never the RichText color markup
            Keys.onPressed: (event) => {
                if (event.matches(StandardKey.Copy)) {
                    copyBuffer.text = edit.selectedText.replace(/\u2029/g, "\n");
                    copyBuffer.selectAll();
                    copyBuffer.copy();
                    event.accepted = true;
                }
            }
        }
    }

    Label {
        anchors.centerIn: parent
        text: "No log entries"
        visible: logViewer.entries.length === 0 && logViewer.logVisible
        opacity: 0.5
    }

    TextEdit {
        id: copyBuffer
        visible: false
        textFormat: TextEdit.PlainText
    }
}
