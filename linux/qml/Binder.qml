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
                text: "Binder"
                color: "#b8b8b8"
                font.pixelSize: 12
                font.letterSpacing: 1.4
                font.capitalization: Font.AllUppercase
            }
        }

        ListView {
            id: list
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            boundsBehavior: Flickable.StopAtBounds
            model: controller.binder_names
            currentIndex: -1

            ScrollBar.vertical: ScrollBar {}

            delegate: ItemDelegate {
                width: list.width
                height: 28
                highlighted: controller.active_doc_id === controller.binder_ids[index]

                contentItem: RowLayout {
                    spacing: 6
                    Item {
                        Layout.preferredWidth: 12 + parseInt(controller.binder_depths[index]) * 14
                    }
                    Label {
                        text: controller.binder_kinds[index] === "Folder" ? "▸" : "•"
                        color: controller.binder_kinds[index] === "Folder" ? "#7a9a7a" : "#808080"
                        font.pixelSize: 12
                    }
                    Label {
                        text: modelData
                        color: controller.binder_kinds[index] === "Folder" ? "#d0d0d0" : "#b8b8b8"
                        font.pixelSize: 13
                        font.bold: controller.binder_kinds[index] === "Folder"
                        Layout.fillWidth: true
                        elide: Text.ElideRight
                    }
                }

                onClicked: {
                    if (controller.binder_kinds[index] === "Document") {
                        controller.select_document(controller.binder_ids[index])
                    }
                }
            }

            Label {
                anchors.centerIn: parent
                visible: list.count === 0
                text: "No project open.\nFile → Open Project..."
                horizontalAlignment: Text.AlignHCenter
                color: "#707070"
                font.pixelSize: 12
            }
        }
    }
}
