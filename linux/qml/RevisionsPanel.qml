import QtQuick
import QtQuick.Controls
import QtQuick.Controls.Material
import QtQuick.Layouts

Rectangle {
    id: root
    color: "#232323"
    property var controller
    property var revisions: []

    function refresh() {
        try {
            revisions = JSON.parse(controller.list_revisions_json())
        } catch(e) {
            revisions = []
        }
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: 0

        Rectangle {
            Layout.fillWidth: true
            Layout.preferredHeight: 34
            color: "#2b2b2b"
            RowLayout {
                anchors.fill: parent
                anchors.leftMargin: 12
                anchors.rightMargin: 8
                Label {
                    text: "Revisions"
                    color: "#b8b8b8"
                    font.pixelSize: 12
                    font.letterSpacing: 1.4
                    font.capitalization: Font.AllUppercase
                }
                Item { Layout.fillWidth: true }
                ToolButton {
                    text: "↻"
                    font.pixelSize: 16
                    onClicked: root.refresh()
                    ToolTip.text: "Refresh"
                    ToolTip.visible: hovered
                }
            }
        }

        ListView {
            id: revList
            Layout.fillWidth: true
            Layout.fillHeight: true
            clip: true
            model: revisions
            currentIndex: -1

            ScrollBar.vertical: ScrollBar {}

            delegate: ItemDelegate {
                width: revList.width
                highlighted: revList.currentIndex === index
                onClicked: revList.currentIndex = index

                contentItem: ColumnLayout {
                    spacing: 2
                    Label {
                        text: modelData.message
                        color: "#d8d8d8"
                        font.pixelSize: 13
                        elide: Text.ElideRight
                        Layout.fillWidth: true
                    }
                    Label {
                        text: modelData.short_id + "  " + modelData.timestamp
                        color: "#707070"
                        font.pixelSize: 11
                        font.family: "monospace"
                    }
                }
            }

            Label {
                anchors.centerIn: parent
                visible: revList.count === 0
                text: "No revisions yet.\nSave a revision to see it here."
                horizontalAlignment: Text.AlignHCenter
                color: "#707070"
                font.pixelSize: 12
            }
        }

        RowLayout {
            Layout.fillWidth: true
            Layout.preferredHeight: 40
            Layout.leftMargin: 8
            Layout.rightMargin: 8
            spacing: 8

            Button {
                text: "Restore"
                enabled: revList.currentIndex >= 0
                Material.accent: "#d2691e"
                onClicked: {
                    if (revList.currentIndex < 0) return
                    var rev = revisions[revList.currentIndex]
                    var err = controller.restore_revision_by_id(rev.id)
                    if (err.length > 0) console.log("Restore error: " + err)
                    else root.refresh()
                }
            }
            Item { Layout.fillWidth: true }
        }
    }

    Component.onCompleted: root.refresh()
}
