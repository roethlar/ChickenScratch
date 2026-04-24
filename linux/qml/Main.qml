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

    property bool showRevisions: false
    property var statsData: ({})

    AppController { id: controller }

    // ── Actions ──────────────────────────────────────────────────────────────

    Action {
        id: openAction
        text: "Open Project…"
        shortcut: "Ctrl+O"
        onTriggered: mainOpenDialog.open()
    }
    Action {
        id: newProjectAction
        text: "New Project…"
        shortcut: "Ctrl+Shift+N"
        onTriggered: mainNewProjectDialog.open()
    }
    Action {
        id: newDocAction
        text: "New Document…"
        shortcut: "Ctrl+N"
        enabled: !controller.show_welcome
        onTriggered: newDocDialog.open()
    }
    Action {
        id: newFolderAction
        text: "New Folder…"
        enabled: !controller.show_welcome
        onTriggered: newFolderDialog.open()
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
        id: saveRevisionAction
        text: "Save Revision…"
        shortcut: "Ctrl+Shift+S"
        enabled: !controller.show_welcome
        onTriggered: saveRevDialog.open()
    }
    Action {
        id: statsAction
        text: "Statistics…"
        enabled: !controller.show_welcome
        onTriggered: {
            try { root.statsData = JSON.parse(controller.get_stats_json()) }
            catch(e) { root.statsData = {} }
            statsDialog.open()
        }
    }
    Action {
        id: quitAction
        text: "Quit"
        shortcut: "Ctrl+Q"
        onTriggered: Qt.quit()
    }
    Action {
        id: toggleBinderAction
        text: "Toggle Binder"
        shortcut: "Ctrl+B"
        checkable: true
        checked: true
        onTriggered: binder.visible = toggleBinderAction.checked
    }
    Action {
        id: toggleInspectorAction
        text: "Toggle Inspector"
        shortcut: "Ctrl+I"
        checkable: true
        checked: true
        onTriggered: inspector.visible = toggleInspectorAction.checked
    }
    Action {
        id: toggleRevisionsAction
        text: "Toggle Revisions"
        shortcut: "Ctrl+R"
        checkable: true
        checked: false
        onTriggered: {
            root.showRevisions = toggleRevisionsAction.checked
            if (root.showRevisions) revPanel.refresh()
        }
    }

    // ── Menu bar ─────────────────────────────────────────────────────────────

    menuBar: MenuBar {
        Menu {
            title: "&File"
            MenuItem { action: openAction }
            MenuItem { action: newProjectAction }
            MenuSeparator {}
            MenuItem { action: newDocAction }
            MenuItem { action: newFolderAction }
            MenuSeparator {}
            MenuItem { action: saveAction }
            MenuItem { action: saveRevisionAction }
            MenuSeparator {}
            MenuItem { action: statsAction }
            MenuSeparator {}
            MenuItem { action: quitAction }
        }
        Menu {
            title: "&View"
            MenuItem { action: toggleBinderAction }
            MenuItem { action: toggleInspectorAction }
            MenuItem { action: toggleRevisionsAction }
        }
    }

    // ── Top-level open/new dialogs (also reachable from menu when welcome is up) ──

    FolderDialog {
        id: mainOpenDialog
        title: "Open .chikn project"
        currentFolder: "file://" + controller.home_dir()
        onAccepted: {
            var local = selectedFolder.toString().replace(/^file:\/\//, "")
            var err = controller.open_project(local)
            if (err.length > 0) errorBanner.show(err)
        }
    }

    Dialog {
        id: mainNewProjectDialog
        title: "New Project"
        standardButtons: Dialog.Ok | Dialog.Cancel
        modal: true
        anchors.centerIn: parent
        width: 420

        ColumnLayout {
            width: parent.width
            spacing: 12
            Label { text: "Project name:" }
            TextField {
                id: mnpNameField
                Layout.fillWidth: true
                placeholderText: "My Novel"
                Keys.onReturnPressed: mainNewProjectDialog.accept()
            }
            Label { text: "Location:" }
            TextField {
                id: mnpPathField
                Layout.fillWidth: true
                placeholderText: "/home/user/Documents/MyNovel.chikn"
                text: controller.home_dir() + "/Documents/" + mnpNameField.text + ".chikn"
            }
        }

        onOpened: { mnpNameField.text = ""; mnpNameField.forceActiveFocus() }
        onAccepted: {
            var name = mnpNameField.text.trim()
            var path = mnpPathField.text.trim()
            if (name.length === 0 || path.length === 0) return
            var err = controller.create_project(path, name)
            if (err.length > 0) errorBanner.show(err)
        }
    }

    // ── New Document dialog ───────────────────────────────────────────────────

    Dialog {
        id: newDocDialog
        title: "New Document"
        standardButtons: Dialog.Ok | Dialog.Cancel
        modal: true
        anchors.centerIn: parent
        width: 380

        ColumnLayout {
            width: parent.width
            spacing: 12
            Label { text: "Document name:" }
            TextField {
                id: newDocNameField
                Layout.fillWidth: true
                placeholderText: "Chapter One"
                Keys.onReturnPressed: newDocDialog.accept()
            }
        }

        onOpened: { newDocNameField.text = ""; newDocNameField.forceActiveFocus() }
        onAccepted: {
            var name = newDocNameField.text.trim()
            if (name.length === 0) return
            var err = controller.new_document(name, "")
            if (err.length > 0) errorBanner.show(err)
        }
    }

    // ── New Folder dialog ─────────────────────────────────────────────────────

    Dialog {
        id: newFolderDialog
        title: "New Folder"
        standardButtons: Dialog.Ok | Dialog.Cancel
        modal: true
        anchors.centerIn: parent
        width: 380

        ColumnLayout {
            width: parent.width
            spacing: 12
            Label { text: "Folder name:" }
            TextField {
                id: newFolderNameField
                Layout.fillWidth: true
                placeholderText: "Part One"
                Keys.onReturnPressed: newFolderDialog.accept()
            }
        }

        onOpened: { newFolderNameField.text = ""; newFolderNameField.forceActiveFocus() }
        onAccepted: {
            var name = newFolderNameField.text.trim()
            if (name.length === 0) return
            var err = controller.new_folder(name, "")
            if (err.length > 0) errorBanner.show(err)
        }
    }

    // ── Save Revision dialog ──────────────────────────────────────────────────

    Dialog {
        id: saveRevDialog
        title: "Save Revision"
        standardButtons: Dialog.Ok | Dialog.Cancel
        modal: true
        anchors.centerIn: parent
        width: 420

        ColumnLayout {
            width: parent.width
            spacing: 12
            Label { text: "Describe this revision:" }
            TextField {
                id: revMsgField
                Layout.fillWidth: true
                placeholderText: "e.g. Finished chapter three"
                Keys.onReturnPressed: saveRevDialog.accept()
            }
        }

        onOpened: { revMsgField.text = ""; revMsgField.forceActiveFocus() }
        onAccepted: {
            var msg = revMsgField.text.trim()
            var err = controller.save_revision_from_msg(msg.length > 0 ? msg : "Manual save")
            if (err.length > 0) errorBanner.show(err)
            else if (root.showRevisions) revPanel.refresh()
        }
    }

    // ── Statistics dialog ─────────────────────────────────────────────────────

    Dialog {
        id: statsDialog
        title: "Statistics"
        standardButtons: Dialog.Close
        modal: true
        anchors.centerIn: parent
        width: 500
        height: 440

        ColumnLayout {
            anchors.fill: parent
            spacing: 10

            // Summary tiles
            RowLayout {
                Layout.fillWidth: true
                spacing: 8

                Repeater {
                    model: [
                        { label: "Words",   value: root.statsData.total_words   || 0 },
                        { label: "Pages",   value: root.statsData.page_count    || 0 },
                        { label: "Min.",    value: root.statsData.reading_minutes || 0 },
                        { label: "Docs",    value: root.statsData.doc_count     || 0 },
                    ]
                    delegate: Rectangle {
                        Layout.fillWidth: true
                        height: 64
                        color: "#2b2b2b"
                        radius: 4
                        ColumnLayout {
                            anchors.centerIn: parent
                            spacing: 2
                            Label {
                                Layout.alignment: Qt.AlignHCenter
                                text: modelData.value.toString()
                                font.pixelSize: 24
                                font.weight: Font.Medium
                                color: "#d2691e"
                            }
                            Label {
                                Layout.alignment: Qt.AlignHCenter
                                text: modelData.label
                                font.pixelSize: 11
                                color: "#9a9a9a"
                            }
                        }
                    }
                }
            }

            // Per-document table
            Rectangle {
                Layout.fillWidth: true
                Layout.fillHeight: true
                color: "#1e1e1e"
                radius: 4

                ColumnLayout {
                    anchors.fill: parent
                    spacing: 0

                    // Header
                    Rectangle {
                        Layout.fillWidth: true
                        height: 26
                        color: "#252525"
                        radius: 4
                        RowLayout {
                            anchors.fill: parent
                            anchors.leftMargin: 10
                            anchors.rightMargin: 10
                            Label {
                                text: "Document"
                                color: "#707070"
                                font.pixelSize: 11
                                font.capitalization: Font.AllUppercase
                                font.letterSpacing: 1.2
                                Layout.fillWidth: true
                            }
                            Label {
                                text: "Words"
                                color: "#707070"
                                font.pixelSize: 11
                                font.capitalization: Font.AllUppercase
                                font.letterSpacing: 1.2
                                Layout.preferredWidth: 70
                                horizontalAlignment: Text.AlignRight
                            }
                        }
                    }

                    ListView {
                        Layout.fillWidth: true
                        Layout.fillHeight: true
                        clip: true
                        model: root.statsData.docs || []
                        ScrollBar.vertical: ScrollBar {}

                        delegate: Rectangle {
                            width: parent ? parent.width : 0
                            height: 30
                            color: index % 2 === 0 ? "#1e1e1e" : "#222222"
                            RowLayout {
                                anchors.fill: parent
                                anchors.leftMargin: 10
                                anchors.rightMargin: 10
                                Label {
                                    text: modelData.name
                                    color: "#c8c8c8"
                                    font.pixelSize: 13
                                    elide: Text.ElideRight
                                    Layout.fillWidth: true
                                }
                                Label {
                                    text: modelData.words.toString()
                                    color: "#9a9a9a"
                                    font.pixelSize: 12
                                    Layout.preferredWidth: 70
                                    horizontalAlignment: Text.AlignRight
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // ── Error banner ─────────────────────────────────────────────────────────

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

    // ── Main content ─────────────────────────────────────────────────────────

    Item {
        anchors.fill: parent
        anchors.topMargin: errorBanner.height

        SplitView {
            id: splitter
            anchors.fill: parent
            orientation: Qt.Horizontal
            visible: !controller.show_welcome

            Binder {
                id: binder
                controller: controller
                SplitView.preferredWidth: 260
                SplitView.minimumWidth: visible ? 180 : 0
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
                SplitView.minimumWidth: visible ? 180 : 0
            }

            RevisionsPanel {
                id: revPanel
                controller: controller
                visible: root.showRevisions
                SplitView.preferredWidth: 240
                SplitView.minimumWidth: visible ? 180 : 0
            }
        }

        Welcome {
            id: welcomeOverlay
            anchors.fill: parent
            visible: controller.show_welcome
            controller: controller
            z: 50
        }
    }

    // ── Status bar ────────────────────────────────────────────────────────────

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
