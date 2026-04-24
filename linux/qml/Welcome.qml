import QtQuick
import QtQuick.Controls
import QtQuick.Controls.Material
import QtQuick.Layouts
import QtQuick.Dialogs

Rectangle {
    id: root
    color: "#1a1a1a"
    property var controller

    signal projectOpened()

    ColumnLayout {
        anchors.centerIn: parent
        spacing: 24
        width: 480

        Label {
            Layout.alignment: Qt.AlignHCenter
            text: "ChickenScratch"
            font.pixelSize: 36
            font.weight: Font.Medium
            color: "#e8e8e8"
        }
        Label {
            Layout.alignment: Qt.AlignHCenter
            text: "A writing app for writers."
            font.pixelSize: 16
            color: "#9a9a9a"
        }

        RowLayout {
            Layout.alignment: Qt.AlignHCenter
            spacing: 12

            Button {
                text: "Open Project…"
                Material.accent: "#d2691e"
                highlighted: true
                onClicked: openDialog.open()
            }
            Button {
                text: "New Project…"
                onClicked: newProjectDialog.open()
            }
        }

        // Recent projects
        Repeater {
            model: {
                try {
                    var arr = JSON.parse(controller.recent_projects_json)
                    return arr
                } catch(e) { return [] }
            }
            delegate: Button {
                Layout.fillWidth: true
                contentItem: ColumnLayout {
                    spacing: 2
                    Label { text: modelData.name; font.pixelSize: 14; color: "#d8d8d8" }
                    Label { text: modelData.path; font.pixelSize: 11; color: "#707070"; elide: Text.ElideMiddle; Layout.fillWidth: true }
                }
                onClicked: {
                    var err = controller.open_project(modelData.path)
                    if (err.length > 0) {
                        errorLabel.text = err
                    } else {
                        root.projectOpened()
                    }
                }
            }
        }

        Label {
            id: errorLabel
            Layout.alignment: Qt.AlignHCenter
            color: "#ffaaaa"
            visible: text.length > 0
            font.pixelSize: 12
        }
    }

    FolderDialog {
        id: openDialog
        title: "Open .chikn project"
        currentFolder: "file://" + controller.home_dir()
        onAccepted: {
            var local = selectedFolder.toString().replace(/^file:\/\//, "")
            var err = controller.open_project(local)
            if (err.length > 0) errorLabel.text = err
            else root.projectOpened()
        }
    }

    Dialog {
        id: newProjectDialog
        title: "New Project"
        standardButtons: Dialog.Ok | Dialog.Cancel
        modal: true
        anchors.centerIn: parent
        width: 400

        ColumnLayout {
            width: parent.width
            spacing: 12

            Label { text: "Project name:" }
            TextField {
                id: projNameField
                Layout.fillWidth: true
                placeholderText: "My Novel"
            }
            Label { text: "Location:" }
            RowLayout {
                Layout.fillWidth: true
                TextField {
                    id: projPathField
                    Layout.fillWidth: true
                    placeholderText: "/home/user/Documents/MyNovel.chikn"
                    text: controller.home_dir() + "/Documents/" + projNameField.text + ".chikn"
                }
            }
        }

        onAccepted: {
            var name = projNameField.text.trim()
            var path = projPathField.text.trim()
            if (name.length === 0 || path.length === 0) return
            var err = controller.create_project(path, name)
            if (err.length > 0) errorLabel.text = err
            else root.projectOpened()
        }
    }
}
