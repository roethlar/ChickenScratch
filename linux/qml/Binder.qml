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
                id: itemDelegate
                width: list.width
                height: 28
                highlighted: controller.active_doc_id === controller.binder_ids[index]

                readonly property bool isFolder: controller.binder_kinds[index] === "Folder"
                readonly property bool hasChildren: controller.binder_has_children[index] === "1"
                readonly property bool isExpanded: controller.binder_expanded[index] === "1"
                readonly property int depth: parseInt(controller.binder_depths[index])
                readonly property string nodeId: controller.binder_ids[index]

                contentItem: RowLayout {
                    spacing: 4
                    Item {
                        Layout.preferredWidth: 8 + itemDelegate.depth * 14
                    }
                    // Chevron for folders that have children
                    Label {
                        Layout.preferredWidth: 14
                        horizontalAlignment: Text.AlignHCenter
                        text: itemDelegate.isFolder && itemDelegate.hasChildren
                              ? (itemDelegate.isExpanded ? "▾" : "▸")
                              : ""
                        color: "#707070"
                        font.pixelSize: 10
                        MouseArea {
                            anchors.fill: parent
                            visible: itemDelegate.isFolder && itemDelegate.hasChildren
                            cursorShape: Qt.PointingHandCursor
                            onClicked: (mouse) => {
                                mouse.accepted = true
                                controller.toggle_folder(itemDelegate.nodeId)
                            }
                        }
                    }
                    Label {
                        Layout.preferredWidth: 12
                        horizontalAlignment: Text.AlignHCenter
                        text: itemDelegate.isFolder ? "▤" : "•"
                        color: itemDelegate.isFolder ? "#7a9a7a" : "#808080"
                        font.pixelSize: 11
                    }
                    Label {
                        text: modelData
                        color: itemDelegate.isFolder ? "#d0d0d0" : "#b8b8b8"
                        font.pixelSize: 13
                        font.bold: itemDelegate.isFolder
                        Layout.fillWidth: true
                        elide: Text.ElideRight
                    }
                }

                onClicked: {
                    if (!itemDelegate.isFolder) {
                        controller.select_document(itemDelegate.nodeId)
                    } else if (itemDelegate.hasChildren) {
                        controller.toggle_folder(itemDelegate.nodeId)
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
