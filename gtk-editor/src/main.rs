mod app;
mod markdown;
mod project;
mod state;
mod ui;

fn main() -> glib::ExitCode {
    if let Err(err) = app::run() {
        eprintln!("Application error: {err:?}");
        return glib::ExitCode::FAILURE;
    }

    glib::ExitCode::SUCCESS
}
