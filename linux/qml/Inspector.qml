import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Rectangle {
    id: root
    color: "#232323"
    property var controller

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 34
            color: "#2b2b2b"
            Label {
                anchors.verticalCenter: parent.verticalCenter
                anchors.left: parent.left
                anchors.leftMargin: 12
                text: "Inspector"
                color: "#b8b8b8"
                font.pixelSize: 12
                font.letterSpacing: 1.4
                font.capitalization: Font.AllUppercase
            }
        }

        ColumnLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            Layout.margins: 14
            spacing: 10

            Label {
                text: "Title"
                color: "#808080"
                font.pixelSize: 11
                font.letterSpacing: 1.2
                font.capitalization: Font.AllUppercase
            }
            Label {
                Layout.fillWidth: true
                text: controller.active_doc_name.length > 0
                      ? controller.active_doc_name
                      : "—"
                color: "#e8e8e8"
                font.pixelSize: 14
                wrapMode: Label.WordWrap
            }

            Label {
                text: "Words"
                color: "#808080"
                font.pixelSize: 11
                font.letterSpacing: 1.2
                font.capitalization: Font.AllUppercase
                Layout.topMargin: 10
            }
            Label {
                text: controller.active_doc_content.length > 0
                      ? controller.active_doc_content.trim().split(/\s+/).filter(function(w) { return w.length > 0 }).length
                      : "0"
                color: "#e8e8e8"
                font.pixelSize: 14
            }

            Item { Layout.fillHeight: true }

            Label {
                Layout.fillWidth: true
                visible: controller.active_doc_id.length === 0
                text: "Metadata editing, comments, revisions, compile, and AI panels land in follow-up commits."
                color: "#707070"
                font.pixelSize: 11
                wrapMode: Label.WordWrap
            }
        }
    }
}
