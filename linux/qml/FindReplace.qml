import QtQuick
import QtQuick.Controls
import QtQuick.Controls.Material
import QtQuick.Layouts

Rectangle {
    id: root
    property var target           // TextArea we operate on
    property bool replaceMode: false

    signal closed()

    color: "#2b2b2b"
    border.color: "#3a3a3a"
    border.width: 1
    implicitHeight: replaceMode ? 76 : 40
    Behavior on implicitHeight { NumberAnimation { duration: 120 } }

    property string find: ""
    property string replace: ""
    property var matches: []
    property int currentIndex: -1

    function open(withReplace) {
        replaceMode = withReplace
        visible = true
        findField.forceActiveFocus()
        findField.selectAll()
        recompute()
    }

    function close() {
        visible = false
        root.closed()
    }

    function recompute() {
        if (!target || find.length === 0) {
            matches = []
            currentIndex = -1
            return
        }
        var text = target.text
        var needle = find.toLowerCase()
        var hay = text.toLowerCase()
        var found = []
        var idx = 0
        while (idx < hay.length) {
            var at = hay.indexOf(needle, idx)
            if (at < 0) break
            found.push(at)
            idx = at + Math.max(1, needle.length)
        }
        matches = found
        if (found.length === 0) {
            currentIndex = -1
        } else if (currentIndex >= found.length || currentIndex < 0) {
            currentIndex = 0
            selectCurrent()
        }
    }

    function selectCurrent() {
        if (!target || currentIndex < 0 || currentIndex >= matches.length) return
        var start = matches[currentIndex]
        target.select(start, start + find.length)
        target.cursorPosition = start + find.length
    }

    function next() {
        if (matches.length === 0) return
        currentIndex = (currentIndex + 1) % matches.length
        selectCurrent()
    }
    function prev() {
        if (matches.length === 0) return
        currentIndex = (currentIndex - 1 + matches.length) % matches.length
        selectCurrent()
    }
    function replaceCurrent() {
        if (!target || matches.length === 0 || currentIndex < 0) return
        var start = matches[currentIndex]
        target.remove(start, start + find.length)
        target.insert(start, replace)
        target.cursorPosition = start + replace.length
        recompute()
    }
    function replaceAll() {
        if (!target || matches.length === 0) return
        // Replace from end to start so earlier positions stay valid
        for (var i = matches.length - 1; i >= 0; i--) {
            var start = matches[i]
            target.remove(start, start + find.length)
            target.insert(start, replace)
        }
        currentIndex = -1
        recompute()
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 6
        spacing: 4

        RowLayout {
            Layout.fillWidth: true
            spacing: 6

            TextField {
                id: findField
                Layout.fillWidth: true
                placeholderText: "Find..."
                text: root.find
                onTextEdited: { root.find = text; root.recompute() }
                onAccepted: root.next()
                Keys.onPressed: (e) => {
                    if (e.key === Qt.Key_Escape) { root.close(); e.accepted = true }
                    else if (e.key === Qt.Key_Return && (e.modifiers & Qt.ShiftModifier)) {
                        root.prev(); e.accepted = true
                    }
                }
            }
            Label {
                text: root.find.length > 0
                      ? (root.matches.length > 0
                         ? (root.currentIndex + 1) + "/" + root.matches.length
                         : "0/0")
                      : ""
                color: "#9a9a9a"
                font.pixelSize: 12
                Layout.preferredWidth: 50
                horizontalAlignment: Text.AlignRight
            }
            ToolButton {
                text: "↑"
                enabled: root.matches.length > 0
                ToolTip.visible: hovered
                ToolTip.text: "Previous (Shift+Enter)"
                onClicked: root.prev()
            }
            ToolButton {
                text: "↓"
                enabled: root.matches.length > 0
                ToolTip.visible: hovered
                ToolTip.text: "Next (Enter)"
                onClicked: root.next()
            }
            ToolButton {
                text: "⇆"
                checkable: true
                checked: root.replaceMode
                ToolTip.visible: hovered
                ToolTip.text: "Toggle Replace"
                onToggled: root.replaceMode = checked
            }
            ToolButton {
                text: "✕"
                ToolTip.visible: hovered
                ToolTip.text: "Close (Esc)"
                onClicked: root.close()
            }
        }

        RowLayout {
            Layout.fillWidth: true
            visible: root.replaceMode
            spacing: 6

            TextField {
                Layout.fillWidth: true
                placeholderText: "Replace with..."
                text: root.replace
                onTextEdited: root.replace = text
                Keys.onPressed: (e) => {
                    if (e.key === Qt.Key_Escape) { root.close(); e.accepted = true }
                }
            }
            Button {
                text: "Replace"
                enabled: root.matches.length > 0
                onClicked: root.replaceCurrent()
            }
            Button {
                text: "All"
                enabled: root.matches.length > 0
                onClicked: root.replaceAll()
            }
        }
    }
}
