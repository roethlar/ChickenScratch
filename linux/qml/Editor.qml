import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Rectangle {
    id: root
    color: "#1a1a1a"
    property var controller

    Shortcut {
        sequence: "Ctrl+F"
        onActivated: findBar.open(false)
    }
    Shortcut {
        sequence: "Ctrl+H"
        onActivated: findBar.open(true)
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
                anchors.rightMargin: 20
                Label {
                    text: controller.active_doc_name
                    color: "#e8e8e8"
                    font.pixelSize: 14
                    font.bold: true
                    Layout.fillWidth: true
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
                font.family: "Literata, Georgia, serif"
                font.pixelSize: 17
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
