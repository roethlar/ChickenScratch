use cxx_qt_build::{CxxQtBuilder, QmlModule};

fn main() {
    CxxQtBuilder::new()
        .qt_module("Quick")
        .qt_module("QuickControls2")
        .qml_module(QmlModule {
            uri: "com.chikn.linux",
            rust_files: &["src/bridge.rs"],
            qml_files: &[
                "qml/Main.qml",
                "qml/Binder.qml",
                "qml/Editor.qml",
                "qml/FindReplace.qml",
                "qml/Inspector.qml",
                "qml/Welcome.qml",
                "qml/RevisionsPanel.qml",
            ],
            ..Default::default()
        })
        .build();
}
