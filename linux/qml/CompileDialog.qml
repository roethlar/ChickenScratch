import QtQuick
import QtQuick.Controls
import QtQuick.Controls.Material
import QtQuick.Dialogs
import QtQuick.Layouts

Dialog {
    id: root
    title: "Compile Manuscript"
    standardButtons: Dialog.Cancel
    modal: true
    width: 520

    property var controller
    property string statusText: ""
    property bool busy: false

    readonly property var formats: [
        { ext: "docx", label: "Word Document (.docx)" },
        { ext: "pdf",  label: "PDF (.pdf)" },
        { ext: "epub", label: "EPUB (.epub)" },
        { ext: "html", label: "HTML (.html)" },
        { ext: "odt",  label: "OpenDocument (.odt)" }
    ]

    property int formatIndex: 0
    property string outputPath: ""

    function defaultOutputPath() {
        var ext = formats[formatIndex].ext
        var dir = controller.home_dir() + "/Documents"
        var name = controller.project_title.length > 0
                   ? controller.project_title.replace(/[^a-zA-Z0-9_-]+/g, "_")
                   : "manuscript"
        return dir + "/" + name + "." + ext
    }

    onFormatIndexChanged: {
        // Swap extension when format changes if the user hasn't customised the path
        if (outputPath.length === 0 || outputPath.indexOf("/Documents/") >= 0) {
            outputPath = defaultOutputPath()
        }
    }

    onOpened: {
        outputPath = defaultOutputPath()
        statusText = ""
        busy = false
    }

    FileDialog {
        id: savePicker
        fileMode: FileDialog.SaveFile
        title: "Save compiled manuscript"
        currentFile: "file://" + root.outputPath
        nameFilters: [root.formats[root.formatIndex].label, "All files (*)"]
        onAccepted: {
            root.outputPath = selectedFile.toString().replace(/^file:\/\//, "")
        }
    }

    contentItem: ColumnLayout {
        spacing: 12
        width: parent.width

        // Format
        ColumnLayout {
            Layout.fillWidth: true
            spacing: 4
            Label { text: "FORMAT"; color: "#808080"; font.pixelSize: 11; font.letterSpacing: 1.2 }
            ComboBox {
                id: formatBox
                Layout.fillWidth: true
                model: root.formats.map(function(f) { return f.label })
                currentIndex: root.formatIndex
                onActivated: root.formatIndex = currentIndex
            }
        }

        // Output path
        ColumnLayout {
            Layout.fillWidth: true
            spacing: 4
            Label { text: "OUTPUT FILE"; color: "#808080"; font.pixelSize: 11; font.letterSpacing: 1.2 }
            RowLayout {
                Layout.fillWidth: true
                spacing: 6
                TextField {
                    Layout.fillWidth: true
                    text: root.outputPath
                    onEditingFinished: root.outputPath = text
                }
                Button {
                    text: "Browse…"
                    onClicked: savePicker.open()
                }
            }
        }

        // Manuscript format toggle (Shunn preset)
        CheckBox {
            id: manuscriptFormat
            text: "Standard manuscript format (Courier 12pt, double-spaced, 1\" margins)"
            checked: false
        }

        CheckBox {
            id: includeTitlePage
            text: "Include title page"
            checked: true
        }

        // Custom typography (only when manuscript-format is off)
        ColumnLayout {
            Layout.fillWidth: true
            spacing: 8
            visible: !manuscriptFormat.checked

            RowLayout {
                Layout.fillWidth: true
                spacing: 8
                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 2
                    Label { text: "Font"; color: "#9a9a9a"; font.pixelSize: 11 }
                    TextField {
                        id: fontField
                        Layout.fillWidth: true
                        text: "Times New Roman"
                    }
                }
                ColumnLayout {
                    Layout.preferredWidth: 90
                    spacing: 2
                    Label { text: "Size (pt)"; color: "#9a9a9a"; font.pixelSize: 11 }
                    SpinBox {
                        id: fontSizeBox
                        from: 8; to: 24
                        value: 12
                    }
                }
            }

            RowLayout {
                Layout.fillWidth: true
                spacing: 8
                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 2
                    Label { text: "Line spacing"; color: "#9a9a9a"; font.pixelSize: 11 }
                    ComboBox {
                        id: lineSpacingBox
                        Layout.fillWidth: true
                        model: [
                            { label: "Single", value: 1.0 },
                            { label: "1.5",    value: 1.5 },
                            { label: "Double", value: 2.0 }
                        ]
                        textRole: "label"
                        valueRole: "value"
                        currentIndex: 2
                    }
                }
                ColumnLayout {
                    Layout.preferredWidth: 110
                    spacing: 2
                    Label { text: "Margin (in)"; color: "#9a9a9a"; font.pixelSize: 11 }
                    SpinBox {
                        id: marginBox
                        from: 25; to: 200
                        stepSize: 25
                        value: 100
                        property real realValue: value / 100.0
                        textFromValue: function(v, locale) { return (v / 100.0).toFixed(2) }
                        valueFromText: function(t, locale) { return Math.round(parseFloat(t) * 100) }
                    }
                }
            }
        }

        ColumnLayout {
            Layout.fillWidth: true
            spacing: 2
            Label { text: "Section separator (between docs)"; color: "#9a9a9a"; font.pixelSize: 11 }
            TextField {
                id: separatorField
                Layout.fillWidth: true
                text: "# # #"
                placeholderText: "# # # or *** etc."
            }
        }

        // Status / busy / errors
        Label {
            Layout.fillWidth: true
            visible: root.statusText.length > 0
            text: root.statusText
            wrapMode: Label.WordWrap
            color: root.statusText.indexOf("compiled") >= 0 ? "#7a9a7a" : "#d27a6c"
            font.pixelSize: 12
        }

        RowLayout {
            Layout.fillWidth: true
            Layout.topMargin: 8
            spacing: 8
            BusyIndicator {
                running: root.busy
                visible: root.busy
                Layout.preferredWidth: 24
                Layout.preferredHeight: 24
            }
            Item { Layout.fillWidth: true }
            Button {
                text: "Compile"
                Material.background: "#d2691e"
                Material.foreground: "white"
                enabled: !root.busy && root.outputPath.length > 0
                onClicked: {
                    root.busy = true
                    root.statusText = "Compiling…"
                    var fmt = root.formats[root.formatIndex].ext
                    var lineSpacing = lineSpacingBox.currentValue || 2.0
                    var margin = marginBox.value / 100.0
                    var err = controller.compile_project(
                        root.outputPath,
                        fmt,
                        manuscriptFormat.checked,
                        fontField.text,
                        fontSizeBox.value,
                        lineSpacing,
                        margin,
                        separatorField.text,
                        includeTitlePage.checked
                    )
                    root.busy = false
                    if (err.length === 0) {
                        root.statusText = "Successfully compiled to " + root.outputPath
                    } else {
                        root.statusText = "Compile failed: " + err
                    }
                }
            }
        }
    }
}
