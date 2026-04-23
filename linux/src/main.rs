mod bridge;

use cxx_qt_lib::{QGuiApplication, QQmlApplicationEngine, QUrl};

fn main() {
    let mut app = QGuiApplication::new();
    let mut engine = QQmlApplicationEngine::new();

    match engine.as_mut() {
        Some(e) => {
            let url = QUrl::from("qrc:/qt/qml/com/chikn/linux/qml/Main.qml");
            eprintln!("[chikn] loading QML: {}", url.to_string());
            e.load(&url);
        }
        None => {
            eprintln!("[chikn] ERROR: QQmlApplicationEngine::new() returned None");
            std::process::exit(1);
        }
    }

    match app.as_mut() {
        Some(a) => {
            eprintln!("[chikn] entering event loop");
            a.exec();
        }
        None => {
            eprintln!("[chikn] ERROR: QGuiApplication::new() returned None");
            std::process::exit(1);
        }
    }
}
