import QtQuick
import QtQuick.Controls
import QtQuick.Controls.Material
import QtQuick.Layouts

Rectangle {
    id: root
    color: "#232323"
    property var controller

    readonly property bool hasDoc: controller.active_doc_id.length > 0

    // Local editable state — synced from controller on doc change, flushed back on idle.
    property string editTitle: ""
    property string editSynopsis: ""
    property string editLabel: ""
    property string editStatus: ""
    property string editKeywords: ""
    property bool editInclude: true
    property int editTarget: 0
    property string lastDocId: ""
    property bool suspendSync: false

    // Pull controller → local on document switch
    Connections {
        target: controller
        function onActive_doc_idChanged() { root.syncFromController() }
        function onDoc_synopsisChanged() { if (root.lastDocId === controller.active_doc_id) root.syncFromController() }
        function onDoc_labelChanged() { if (root.lastDocId === controller.active_doc_id) root.syncFromController() }
        function onDoc_statusChanged() { if (root.lastDocId === controller.active_doc_id) root.syncFromController() }
        function onDoc_keywordsChanged() { if (root.lastDocId === controller.active_doc_id) root.syncFromController() }
        function onDoc_include_in_compileChanged() { if (root.lastDocId === controller.active_doc_id) root.syncFromController() }
        function onDoc_word_count_targetChanged() { if (root.lastDocId === controller.active_doc_id) root.syncFromController() }
    }

    function syncFromController() {
        suspendSync = true
        editTitle = controller.active_doc_name
        editSynopsis = controller.doc_synopsis
        editLabel = controller.doc_label
        editStatus = controller.doc_status
        editKeywords = controller.doc_keywords
        editInclude = controller.doc_include_in_compile
        editTarget = controller.doc_word_count_target
        lastDocId = controller.active_doc_id
        suspendSync = false
    }

    Component.onCompleted: syncFromController()

    Timer {
        id: metaDebounce
        interval: 1200
        onTriggered: {
            if (!root.hasDoc) return
            controller.save_metadata(
                root.editSynopsis,
                root.editLabel,
                root.editStatus,
                root.editKeywords,
                root.editInclude,
                root.editTarget
            )
        }
    }

    function scheduleSave() {
        if (suspendSync || !hasDoc) return
        metaDebounce.restart()
    }

    readonly property int wordCount: {
        if (controller.active_doc_content.length === 0) return 0
        var words = controller.active_doc_content.trim().split(/\s+/)
        var n = 0
        for (var i = 0; i < words.length; i++) if (words[i].length > 0) n++
        return n
    }

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

        Flickable {
            Layout.fillWidth: true
            Layout.fillHeight: true
            contentWidth: width
            contentHeight: form.implicitHeight
            clip: true
            ScrollBar.vertical: ScrollBar {}

            ColumnLayout {
                id: form
                width: parent.width
                spacing: 14
                anchors.left: parent.left
                anchors.right: parent.right
                anchors.margins: 14

                Label {
                    Layout.fillWidth: true
                    visible: !root.hasDoc
                    text: "Select a document to edit its metadata."
                    color: "#707070"
                    font.pixelSize: 12
                    wrapMode: Label.WordWrap
                }

                // Title (rename)
                ColumnLayout {
                    Layout.fillWidth: true
                    visible: root.hasDoc
                    spacing: 4
                    Label { text: "TITLE"; color: "#808080"; font.pixelSize: 11; font.letterSpacing: 1.2 }
                    TextField {
                        id: titleField
                        Layout.fillWidth: true
                        text: root.editTitle
                        color: "#e8e8e8"
                        font.pixelSize: 14
                        onTextEdited: root.editTitle = text
                        onEditingFinished: {
                            var trimmed = root.editTitle.trim()
                            if (trimmed.length > 0 && trimmed !== controller.active_doc_name) {
                                controller.rename_node(controller.active_doc_id, trimmed)
                            } else {
                                root.editTitle = controller.active_doc_name
                            }
                        }
                    }
                }

                ColumnLayout {
                    Layout.fillWidth: true
                    visible: root.hasDoc
                    spacing: 4
                    Label { text: "SYNOPSIS"; color: "#808080"; font.pixelSize: 11; font.letterSpacing: 1.2 }
                    TextArea {
                        Layout.fillWidth: true
                        Layout.preferredHeight: 80
                        text: root.editSynopsis
                        placeholderText: "What happens in this scene..."
                        wrapMode: TextEdit.Wrap
                        color: "#e8e8e8"
                        font.pixelSize: 13
                        background: Rectangle { color: "#1a1a1a"; radius: 4; border.color: "#3a3a3a"; border.width: 1 }
                        onTextChanged: {
                            if (text !== root.editSynopsis) {
                                root.editSynopsis = text
                                root.scheduleSave()
                            }
                        }
                    }
                }

                ColumnLayout {
                    Layout.fillWidth: true
                    visible: root.hasDoc
                    spacing: 4
                    Label { text: "LABEL"; color: "#808080"; font.pixelSize: 11; font.letterSpacing: 1.2 }
                    ComboBox {
                        Layout.fillWidth: true
                        editable: true
                        model: ["", "Scene", "Chapter", "Outline", "Notes", "Research"]
                        editText: root.editLabel
                        onEditTextChanged: {
                            if (editText !== root.editLabel) {
                                root.editLabel = editText
                                root.scheduleSave()
                            }
                        }
                    }
                }

                ColumnLayout {
                    Layout.fillWidth: true
                    visible: root.hasDoc
                    spacing: 4
                    Label { text: "STATUS"; color: "#808080"; font.pixelSize: 11; font.letterSpacing: 1.2 }
                    ComboBox {
                        Layout.fillWidth: true
                        editable: true
                        model: ["", "Draft", "Revised", "Final", "To Do", "In Progress"]
                        editText: root.editStatus
                        onEditTextChanged: {
                            if (editText !== root.editStatus) {
                                root.editStatus = editText
                                root.scheduleSave()
                            }
                        }
                    }
                }

                ColumnLayout {
                    Layout.fillWidth: true
                    visible: root.hasDoc
                    spacing: 4
                    Label { text: "KEYWORDS"; color: "#808080"; font.pixelSize: 11; font.letterSpacing: 1.2 }
                    TextField {
                        Layout.fillWidth: true
                        text: root.editKeywords
                        placeholderText: "tag1, tag2, tag3"
                        color: "#e8e8e8"
                        onTextEdited: {
                            root.editKeywords = text
                            root.scheduleSave()
                        }
                    }
                }

                RowLayout {
                    Layout.fillWidth: true
                    visible: root.hasDoc
                    spacing: 10
                    CheckBox {
                        text: "Include in Compile"
                        checked: root.editInclude
                        onToggled: {
                            root.editInclude = checked
                            root.scheduleSave()
                        }
                    }
                    Item { Layout.fillWidth: true }
                }

                ColumnLayout {
                    Layout.fillWidth: true
                    visible: root.hasDoc
                    spacing: 4
                    Label { text: "WORDS"; color: "#808080"; font.pixelSize: 11; font.letterSpacing: 1.2 }
                    Label {
                        text: root.editTarget > 0
                              ? root.wordCount + " / " + root.editTarget
                              : root.wordCount.toString()
                        color: "#e8e8e8"
                        font.pixelSize: 14
                    }
                    Rectangle {
                        Layout.fillWidth: true
                        Layout.preferredHeight: 4
                        visible: root.editTarget > 0
                        color: "#1a1a1a"
                        radius: 2
                        Rectangle {
                            height: parent.height
                            width: parent.width * Math.min(1.0, root.wordCount / Math.max(1, root.editTarget))
                            color: root.wordCount >= root.editTarget ? "#7a9a7a" : "#d2691e"
                            radius: 2
                        }
                    }
                    SpinBox {
                        Layout.fillWidth: true
                        from: 0
                        to: 1000000
                        stepSize: 100
                        editable: true
                        value: root.editTarget
                        onValueModified: {
                            if (value !== root.editTarget) {
                                root.editTarget = value
                                root.scheduleSave()
                            }
                        }
                    }
                }

                ColumnLayout {
                    Layout.fillWidth: true
                    visible: root.hasDoc && controller.doc_modified.length > 0
                    spacing: 4
                    Label { text: "MODIFIED"; color: "#808080"; font.pixelSize: 11; font.letterSpacing: 1.2 }
                    Label {
                        text: controller.doc_modified
                        color: "#9a9a9a"
                        font.pixelSize: 12
                        wrapMode: Label.WrapAnywhere
                        Layout.fillWidth: true
                    }
                }
            }
        }
    }
}
