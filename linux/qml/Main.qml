import QtQuick
import QtQuick.Controls
import QtQuick.Controls.Material
import QtQuick.Dialogs
import QtQuick.Layouts
import com.chikn.linux

ApplicationWindow {
    id: root
    visible: true
    width: 1280
    height: 800
    minimumWidth: 720
    minimumHeight: 480
    title: controller.project_title.length > 0
           ? "Chicken Scratch — " + controller.project_title
           : "Chicken Scratch"

    Material.theme: Material.Dark
    Material.accent: "#d2691e"
    Material.primary: "#2a2a2a"
    Material.background: "#1a1a1a"
    Material.foreground: "#e8e8e8"

    AppController { id: controller }

    Action {
        id: openAction
        text: "Open Project..."
        shortcut: "Ctrl+O"
        onTriggered: openDialog.open()
    }
    Action {
        id: saveAction
        text: "Save"
        shortcut: "Ctrl+S"
        enabled: controller.active_doc_id.length > 0
        onTriggered: {
            var err = controller.save()
            if (err.length > 0) errorBanner.show(err)
        }
    }
    Action {
        id: quitAction
        text: "Quit"
        shortcut: "Ctrl+Q"
        onTriggered: Qt.quit()
    }

    FolderDialog {
        id: openDialog
        title: "Open .chikn project"
        currentFolder: "file://" + controller.home_dir()
        onAccepted: {
            var local = selectedFolder.toString().replace(/^file:\/\//, "")
            var err = controller.open_project(local)
            if (err.length > 0) errorBanner.show(err)
        }
    }

    menuBar: MenuBar {
        Menu {
            title: "&File"
            MenuItem { action: openAction }
            MenuItem { action: saveAction }
            MenuSeparator {}
            MenuItem { action: quitAction }
        }
    }

    Rectangle {
        id: errorBanner
        anchors.top: parent.top
        anchors.left: parent.left
        anchors.right: parent.right
        height: label.visible ? 32 : 0
        color: "#5a1c1c"
        z: 100
        Behavior on height { NumberAnimation { duration: 120 } }
        property alias text: label.text
        function show(msg) {
            label.text = msg
            label.visible = true
            dismissTimer.restart()
        }
        Label {
            id: label
            anchors.centerIn: parent
            color: "#ffcccc"
            visible: false
        }
        Timer {
            id: dismissTimer
            interval: 4000
            onTriggered: label.visible = false
        }
    }

    SplitView {
        id: splitter
        anchors.fill: parent
        anchors.topMargin: errorBanner.height
        orientation: Qt.Horizontal

        Binder {
            id: binder
            controller: controller
            SplitView.preferredWidth: 260
            SplitView.minimumWidth: 180
        }

        Editor {
            id: editor
            controller: controller
            SplitView.fillWidth: true
        }

        Inspector {
            id: inspector
            controller: controller
            SplitView.preferredWidth: 260
            SplitView.minimumWidth: 180
        }
    }

    footer: ToolBar {
        height: 28
        RowLayout {
            anchors.fill: parent
            anchors.leftMargin: 12
            anchors.rightMargin: 12
            Label {
                text: controller.active_doc_name.length > 0
                      ? controller.active_doc_name
                      : "No document"
                color: "#9a9a9a"
                font.pixelSize: 12
            }
            Item { Layout.fillWidth: true }
            Label {
                text: controller.save_label
                color: controller.dirty ? "#d2691e" : "#7a9a7a"
                font.pixelSize: 12
            }
        }
    }
}
