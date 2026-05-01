import QtQuick
import QtQuick.Controls
import QtQuick.Controls.Material
import QtQuick.Dialogs
import QtQuick.Layouts

Dialog {
    id: root
    title: "Settings"
    standardButtons: Dialog.Cancel
    modal: true
    width: 580
    height: 540

    property var controller
    property var settings: ({})
    property string statusText: ""
    property string pandocVersion: ""

    function reload() {
        try { settings = JSON.parse(controller.settings_json || "{}") }
        catch(e) { settings = {} }
        if (!settings.general) settings.general = { theme: "dark", recent_projects_limit: 10, pandoc_path: null }
        if (!settings.writing) settings.writing = { font_family: "Literata Variable", font_size: 18, paragraph_style: "block", auto_save_seconds: 2, spell_check: true }
        if (!settings.backup)  settings.backup  = { backup_directory: null, auto_backup_on_close: true, auto_backup_minutes: 30 }
        if (!settings.remote)  settings.remote  = { url: null, username: null, token: null, auto_push_on_revision: false }
        if (!settings.ai)      settings.ai      = { enabled: true, provider: "ollama", endpoint: "http://localhost:11434", api_key: null, model: "llama3.2" }
        if (!settings.compile) settings.compile = { default_format: "docx", font: "Times New Roman", font_size: 12, line_spacing: 2, margin_inches: 1 }
        statusText = ""
        pandocVersion = controller.check_pandoc()
    }

    onOpened: reload()

    function commit() {
        var err = controller.save_settings_json(JSON.stringify(settings))
        if (err.length === 0) {
            statusText = "Saved"
            saveStatusTimer.restart()
        } else {
            statusText = err
        }
    }

    Timer {
        id: saveStatusTimer
        interval: 1500
        onTriggered: root.statusText = ""
    }

    FolderDialog {
        id: backupPicker
        title: "Choose backup folder"
        currentFolder: "file://" + controller.home_dir()
        onAccepted: {
            settings.backup.backup_directory = selectedFolder.toString().replace(/^file:\/\//, "")
            settingsChanged()
        }
    }

    function settingsChanged() {
        // Force re-render of bound fields by reassigning
        var s = settings
        settings = ({})
        settings = s
    }

    contentItem: ColumnLayout {
        spacing: 8
        anchors.margins: 0

        TabBar {
            id: tabs
            Layout.fillWidth: true
            TabButton { text: "General" }
            TabButton { text: "Writing" }
            TabButton { text: "Backup" }
            TabButton { text: "AI" }
            TabButton { text: "Compile" }
            TabButton { text: "Remote" }
        }

        StackLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            currentIndex: tabs.currentIndex

            // ── General ──
            ScrollView {
                clip: true
                ColumnLayout {
                    width: parent.width
                    spacing: 12

                    ColumnLayout {
                        Layout.fillWidth: true
                        Label { text: "Theme"; color: "#9a9a9a"; font.pixelSize: 11 }
                        ComboBox {
                            Layout.fillWidth: true
                            model: ["dark", "light", "sepia"]
                            currentIndex: model.indexOf(root.settings.general && root.settings.general.theme || "dark")
                            onActivated: { root.settings.general.theme = currentText }
                        }
                    }
                    ColumnLayout {
                        Layout.fillWidth: true
                        Label { text: "Recent projects limit"; color: "#9a9a9a"; font.pixelSize: 11 }
                        SpinBox {
                            from: 1; to: 50
                            value: root.settings.general && root.settings.general.recent_projects_limit || 10
                            onValueModified: root.settings.general.recent_projects_limit = value
                        }
                    }
                    ColumnLayout {
                        Layout.fillWidth: true
                        Label { text: "Pandoc path (leave empty for auto-detect)"; color: "#9a9a9a"; font.pixelSize: 11 }
                        TextField {
                            Layout.fillWidth: true
                            text: root.settings.general && root.settings.general.pandoc_path || ""
                            placeholderText: "/usr/bin/pandoc"
                            onEditingFinished: root.settings.general.pandoc_path = text.length > 0 ? text : null
                        }
                        Label {
                            text: "Detected: " + root.pandocVersion
                            color: root.pandocVersion === "Not installed" ? "#d27a6c" : "#7a9a7a"
                            font.pixelSize: 11
                        }
                    }
                    Item { Layout.fillHeight: true }
                }
            }

            // ── Writing ──
            ScrollView {
                clip: true
                ColumnLayout {
                    width: parent.width
                    spacing: 12
                    ColumnLayout {
                        Layout.fillWidth: true
                        Label { text: "Editor font family"; color: "#9a9a9a"; font.pixelSize: 11 }
                        ComboBox {
                            Layout.fillWidth: true
                            editable: true
                            model: ["Literata Variable", "Literata", "Georgia", "Times New Roman", "Palatino", "system-ui"]
                            editText: root.settings.writing && root.settings.writing.font_family || "Literata Variable"
                            onEditTextChanged: { if (root.settings.writing) root.settings.writing.font_family = editText }
                        }
                    }
                    ColumnLayout {
                        Layout.fillWidth: true
                        Label { text: "Font size (px)"; color: "#9a9a9a"; font.pixelSize: 11 }
                        SpinBox {
                            from: 12; to: 28
                            value: Math.round(root.settings.writing && root.settings.writing.font_size || 18)
                            onValueModified: root.settings.writing.font_size = value
                        }
                    }
                    ColumnLayout {
                        Layout.fillWidth: true
                        Label { text: "Paragraph style"; color: "#9a9a9a"; font.pixelSize: 11 }
                        ComboBox {
                            Layout.fillWidth: true
                            model: [
                                { label: "Block (spacing between paragraphs)", value: "block" },
                                { label: "Indent (first-line indent, no spacing)", value: "indent" }
                            ]
                            textRole: "label"
                            valueRole: "value"
                            currentIndex: (root.settings.writing && root.settings.writing.paragraph_style === "indent") ? 1 : 0
                            onActivated: root.settings.writing.paragraph_style = currentValue
                        }
                    }
                    ColumnLayout {
                        Layout.fillWidth: true
                        Label { text: "Auto-save delay (seconds, 0 = off)"; color: "#9a9a9a"; font.pixelSize: 11 }
                        SpinBox {
                            from: 0; to: 30
                            value: root.settings.writing && root.settings.writing.auto_save_seconds !== undefined ? root.settings.writing.auto_save_seconds : 2
                            onValueModified: root.settings.writing.auto_save_seconds = value
                        }
                    }
                    Item { Layout.fillHeight: true }
                }
            }

            // ── Backup ──
            ScrollView {
                clip: true
                ColumnLayout {
                    width: parent.width
                    spacing: 12
                    ColumnLayout {
                        Layout.fillWidth: true
                        Label { text: "Backup directory"; color: "#9a9a9a"; font.pixelSize: 11 }
                        RowLayout {
                            Layout.fillWidth: true
                            TextField {
                                Layout.fillWidth: true
                                text: root.settings.backup && root.settings.backup.backup_directory || ""
                                placeholderText: "~/ChickenScratchBackups"
                                onEditingFinished: root.settings.backup.backup_directory = text.length > 0 ? text : null
                            }
                            Button { text: "Browse…"; onClicked: backupPicker.open() }
                        }
                        Label {
                            text: "Each project gets a git mirror in this folder. Set this to a cloud-synced folder for automatic offsite backup."
                            color: "#707070"
                            font.pixelSize: 11
                            wrapMode: Label.WordWrap
                            Layout.fillWidth: true
                        }
                    }
                    CheckBox {
                        text: "Auto-backup on close"
                        checked: root.settings.backup && root.settings.backup.auto_backup_on_close || false
                        onToggled: root.settings.backup.auto_backup_on_close = checked
                    }
                    ColumnLayout {
                        Layout.fillWidth: true
                        Label { text: "Auto-backup interval (minutes)"; color: "#9a9a9a"; font.pixelSize: 11 }
                        SpinBox {
                            from: 5; to: 240
                            stepSize: 5
                            value: root.settings.backup && root.settings.backup.auto_backup_minutes || 30
                            onValueModified: root.settings.backup.auto_backup_minutes = value
                        }
                    }
                    Item { Layout.fillHeight: true }
                }
            }

            // ── AI ──
            ScrollView {
                clip: true
                ColumnLayout {
                    width: parent.width
                    spacing: 12
                    CheckBox {
                        text: "Enable AI features"
                        checked: root.settings.ai && root.settings.ai.enabled || false
                        onToggled: root.settings.ai.enabled = checked
                    }
                    ColumnLayout {
                        Layout.fillWidth: true
                        enabled: root.settings.ai && root.settings.ai.enabled
                        Label { text: "Provider"; color: "#9a9a9a"; font.pixelSize: 11 }
                        ComboBox {
                            Layout.fillWidth: true
                            model: ["ollama", "anthropic", "openai"]
                            currentIndex: model.indexOf(root.settings.ai && root.settings.ai.provider || "ollama")
                            onActivated: root.settings.ai.provider = currentText
                        }
                    }
                    ColumnLayout {
                        Layout.fillWidth: true
                        enabled: root.settings.ai && root.settings.ai.enabled
                        Label { text: "Model"; color: "#9a9a9a"; font.pixelSize: 11 }
                        TextField {
                            Layout.fillWidth: true
                            text: root.settings.ai && root.settings.ai.model || ""
                            placeholderText: "llama3.2 / claude-sonnet-4-6 / gpt-4o"
                            onEditingFinished: root.settings.ai.model = text
                        }
                    }
                    ColumnLayout {
                        Layout.fillWidth: true
                        enabled: root.settings.ai && root.settings.ai.enabled
                        Label { text: "Endpoint URL"; color: "#9a9a9a"; font.pixelSize: 11 }
                        TextField {
                            Layout.fillWidth: true
                            text: root.settings.ai && root.settings.ai.endpoint || ""
                            placeholderText: "http://localhost:11434"
                            onEditingFinished: root.settings.ai.endpoint = text.length > 0 ? text : null
                        }
                    }
                    ColumnLayout {
                        Layout.fillWidth: true
                        enabled: root.settings.ai && root.settings.ai.enabled && root.settings.ai.provider !== "ollama"
                        Label { text: "API key"; color: "#9a9a9a"; font.pixelSize: 11 }
                        TextField {
                            Layout.fillWidth: true
                            text: root.settings.ai && root.settings.ai.api_key || ""
                            echoMode: TextInput.Password
                            placeholderText: "sk-…"
                            onEditingFinished: root.settings.ai.api_key = text.length > 0 ? text : null
                        }
                    }
                    Item { Layout.fillHeight: true }
                }
            }

            // ── Compile ──
            ScrollView {
                clip: true
                ColumnLayout {
                    width: parent.width
                    spacing: 12
                    ColumnLayout {
                        Layout.fillWidth: true
                        Label { text: "Default export format"; color: "#9a9a9a"; font.pixelSize: 11 }
                        ComboBox {
                            Layout.fillWidth: true
                            model: ["docx", "pdf", "epub", "html", "odt"]
                            currentIndex: model.indexOf(root.settings.compile && root.settings.compile.default_format || "docx")
                            onActivated: root.settings.compile.default_format = currentText
                        }
                    }
                    ColumnLayout {
                        Layout.fillWidth: true
                        Label { text: "Manuscript font"; color: "#9a9a9a"; font.pixelSize: 11 }
                        TextField {
                            Layout.fillWidth: true
                            text: root.settings.compile && root.settings.compile.font || "Times New Roman"
                            onEditingFinished: root.settings.compile.font = text
                        }
                    }
                    RowLayout {
                        Layout.fillWidth: true
                        spacing: 12
                        ColumnLayout {
                            Layout.fillWidth: true
                            Label { text: "Font size (pt)"; color: "#9a9a9a"; font.pixelSize: 11 }
                            SpinBox {
                                from: 8; to: 18
                                value: Math.round(root.settings.compile && root.settings.compile.font_size || 12)
                                onValueModified: root.settings.compile.font_size = value
                            }
                        }
                        ColumnLayout {
                            Layout.fillWidth: true
                            Label { text: "Line spacing"; color: "#9a9a9a"; font.pixelSize: 11 }
                            ComboBox {
                                Layout.fillWidth: true
                                model: [
                                    { label: "Single", value: 1.0 },
                                    { label: "1.5",    value: 1.5 },
                                    { label: "Double", value: 2.0 }
                                ]
                                textRole: "label"
                                valueRole: "value"
                                currentIndex: {
                                    var v = root.settings.compile && root.settings.compile.line_spacing || 2.0
                                    return v <= 1.0 ? 0 : v <= 1.5 ? 1 : 2
                                }
                                onActivated: root.settings.compile.line_spacing = currentValue
                            }
                        }
                    }
                    ColumnLayout {
                        Layout.fillWidth: true
                        Label { text: "Margins (inches)"; color: "#9a9a9a"; font.pixelSize: 11 }
                        SpinBox {
                            from: 25; to: 200
                            stepSize: 25
                            value: Math.round((root.settings.compile && root.settings.compile.margin_inches || 1.0) * 100)
                            textFromValue: function(v) { return (v / 100.0).toFixed(2) }
                            valueFromText: function(t) { return Math.round(parseFloat(t) * 100) }
                            onValueModified: root.settings.compile.margin_inches = value / 100.0
                        }
                    }
                    Item { Layout.fillHeight: true }
                }
            }

            // ── Remote ──
            ScrollView {
                clip: true
                ColumnLayout {
                    width: parent.width
                    spacing: 12
                    ColumnLayout {
                        Layout.fillWidth: true
                        Label { text: "Git URL"; color: "#9a9a9a"; font.pixelSize: 11 }
                        TextField {
                            Layout.fillWidth: true
                            text: root.settings.remote && root.settings.remote.url || ""
                            placeholderText: "https://github.com/you/your-novel.git"
                            onEditingFinished: root.settings.remote.url = text.length > 0 ? text : null
                        }
                    }
                    ColumnLayout {
                        Layout.fillWidth: true
                        Label { text: "Username"; color: "#9a9a9a"; font.pixelSize: 11 }
                        TextField {
                            Layout.fillWidth: true
                            text: root.settings.remote && root.settings.remote.username || ""
                            onEditingFinished: root.settings.remote.username = text.length > 0 ? text : null
                        }
                    }
                    ColumnLayout {
                        Layout.fillWidth: true
                        Label { text: "Token (HTTPS PAT)"; color: "#9a9a9a"; font.pixelSize: 11 }
                        TextField {
                            Layout.fillWidth: true
                            echoMode: TextInput.Password
                            text: root.settings.remote && root.settings.remote.token || ""
                            placeholderText: "ghp_…"
                            onEditingFinished: root.settings.remote.token = text.length > 0 ? text : null
                        }
                        Label {
                            Layout.fillWidth: true
                            text: "Stored in plaintext at ~/.config/chickenscratch/settings.json."
                            color: "#707070"
                            font.pixelSize: 11
                            wrapMode: Label.WordWrap
                        }
                    }
                    CheckBox {
                        text: "Auto-push on named revision"
                        checked: root.settings.remote && root.settings.remote.auto_push_on_revision || false
                        onToggled: root.settings.remote.auto_push_on_revision = checked
                    }
                    Item { Layout.fillHeight: true }
                }
            }
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 8
            Label {
                text: root.statusText
                color: root.statusText === "Saved" ? "#7a9a7a" : "#d27a6c"
                font.pixelSize: 12
                Layout.fillWidth: true
            }
            Button {
                text: "Save"
                Material.background: "#d2691e"
                Material.foreground: "white"
                onClicked: root.commit()
            }
        }
    }
}
