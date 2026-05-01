import QtQuick
import QtQuick.Controls
import QtQuick.Controls.Material
import QtQuick.Layouts

Rectangle {
    id: root
    color: "#1a1a1a"
    property var controller

    function selectRange(start, end) {
        if (start < 0 || end < 0) return
        textArea.cursorPosition = start
        textArea.select(start, end)
        textArea.forceActiveFocus()
    }

    Timer {
        id: autoSaveTimer
        interval: Math.max(500, (controller.auto_save_seconds || 0) * 1000)
        repeat: false
        onTriggered: {
            if (controller.auto_save_seconds > 0
                && controller.dirty
                && controller.active_doc_id.length > 0) {
                controller.save()
            }
        }
    }

    Shortcut {
        sequence: "Ctrl+F"
        onActivated: findBar.open(false)
    }
    Shortcut {
        sequence: "Ctrl+H"
        onActivated: findBar.open(true)
    }
    Shortcut {
        sequence: "Ctrl+;"
        enabled: controller.active_doc_id.length > 0 && textArea.selectionEnd > textArea.selectionStart
        onActivated: addCommentDialog.open()
    }
    Shortcut {
        sequence: "Ctrl+Shift+F"
        enabled: controller.active_doc_id.length > 0
        onActivated: addFootnoteDialog.open()
    }

    Dialog {
        id: addCommentDialog
        title: "New Comment"
        standardButtons: Dialog.Ok | Dialog.Cancel
        modal: true
        anchors.centerIn: parent
        width: 420

        property int selStart: 0
        property int selEnd: 0

        ColumnLayout {
            width: parent.width
            spacing: 10
            Label {
                text: "Anchor: " + (textArea.selectedText.length > 80
                       ? textArea.selectedText.substring(0, 77) + "…"
                       : textArea.selectedText)
                color: "#9a9a9a"
                font.pixelSize: 12
                wrapMode: Label.WordWrap
                Layout.fillWidth: true
            }
            Label { text: "Comment:"; color: "#b8b8b8"; font.pixelSize: 12 }
            TextArea {
                id: addCommentBody
                Layout.fillWidth: true
                Layout.preferredHeight: 80
                placeholderText: "What about this passage?"
                wrapMode: TextEdit.Wrap
                background: Rectangle { color: "#1a1a1a"; radius: 4; border.color: "#3a3a3a"; border.width: 1 }
            }
        }

        onOpened: {
            // Snapshot selection before the dialog steals focus
            addCommentDialog.selStart = textArea.selectionStart
            addCommentDialog.selEnd = textArea.selectionEnd
            addCommentBody.text = ""
            addCommentBody.forceActiveFocus()
        }
        onAccepted: {
            controller.add_comment(addCommentBody.text, addCommentDialog.selStart, addCommentDialog.selEnd)
        }
    }

    Dialog {
        id: addFootnoteDialog
        title: "New Footnote"
        standardButtons: Dialog.Ok | Dialog.Cancel
        modal: true
        anchors.centerIn: parent
        width: 420

        property int cursorPos: 0

        ColumnLayout {
            width: parent.width
            spacing: 10
            Label { text: "Footnote text:"; color: "#b8b8b8"; font.pixelSize: 12 }
            TextArea {
                id: addFootnoteBody
                Layout.fillWidth: true
                Layout.preferredHeight: 80
                placeholderText: "The footnote body..."
                wrapMode: TextEdit.Wrap
                background: Rectangle { color: "#1a1a1a"; radius: 4; border.color: "#3a3a3a"; border.width: 1 }
            }
        }

        onOpened: {
            addFootnoteDialog.cursorPos = textArea.cursorPosition
            addFootnoteBody.text = ""
            addFootnoteBody.forceActiveFocus()
        }
        onAccepted: {
            if (addFootnoteBody.text.trim().length > 0) {
                controller.add_footnote(addFootnoteBody.text.trim(), addFootnoteDialog.cursorPos)
            }
        }
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 40
            color: "#202020"
            visible: controller.active_doc_id.length > 0
            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 20
                anchors.rightMargin: 12
                spacing: 8
                Label {
                    text: controller.active_doc_name
                    color: "#e8e8e8"
                    font.pixelSize: 14
                    font.bold: true
                    Layout.fillWidth: true
                }
                ToolButton {
                    text: "💬"
                    enabled: textArea.selectionEnd > textArea.selectionStart
                    ToolTip.text: "Add comment on selection (Ctrl+;)"
                    ToolTip.visible: hovered
                    implicitWidth: 32
                    implicitHeight: 32
                    onClicked: addCommentDialog.open()
                }
                ToolButton {
                    text: "ⁿ"
                    font.pixelSize: 16
                    ToolTip.text: "Insert footnote at cursor (Ctrl+Shift+F)"
                    ToolTip.visible: hovered
                    implicitWidth: 32
                    implicitHeight: 32
                    onClicked: addFootnoteDialog.open()
                }
            }
        }

        FindReplace {
            id: findBar
            Layout.fillWidth: true
            target: textArea
            visible: false
        }

        Flickable {
            id: scroll
            Layout.fillWidth: true
            Layout.fillHeight: true
            contentWidth: textArea.width
            contentHeight: textArea.contentHeight
            clip: true
            flickableDirection: Flickable.VerticalFlick
            ScrollBar.vertical: ScrollBar {}

            TextArea.flickable: TextArea {
                id: textArea
                placeholderText: controller.active_doc_id.length === 0
                                 ? "Open a project and select a document to start writing."
                                 : "Start writing..."
                text: controller.active_doc_content
                wrapMode: TextEdit.Wrap
                selectByMouse: true
                font.family: controller.editor_font_family.length > 0
                             ? controller.editor_font_family + ", Literata, Georgia, serif"
                             : "Literata, Georgia, serif"
                font.pixelSize: controller.editor_font_size > 0 ? controller.editor_font_size : 17
                color: "#e8e8e8"
                background: null
                leftPadding: 80
                rightPadding: 80
                topPadding: 40
                bottomPadding: 80
                textFormat: TextEdit.PlainText

                readOnly: controller.active_doc_id.length === 0

                onTextChanged: {
                    if (!readOnly && text !== controller.active_doc_content) {
                        controller.update_content(text)
                        if (controller.auto_save_seconds > 0) {
                            autoSaveTimer.restart()
                        }
                    }
                }

                Keys.onPressed: (event) => {
                    if ((event.modifiers & Qt.ControlModifier) && event.key === Qt.Key_S) {
                        controller.save()
                        event.accepted = true
                    }
                }
            }
        }
    }
}
