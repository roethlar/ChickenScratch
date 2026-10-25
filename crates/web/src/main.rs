//! `chikn-web` — run the ChickenScratch UI as a self-hosted web app.
//!
//! ```text
//! chikn-web --root ~/Notes                       # localhost only
//! chikn-web --root ~/Notes --bind 0.0.0.0:7391   # reachable over your VPN
//! chikn-web                                      # settings from web.toml
//! chikn-web --show-config                        # what would be used, and why
//! ```
//!
//! Settings live in `~/.config/chickenscratch/web.toml` (`config` module);
//! flags and `CHIKN_WEB_*` environment variables override it.
//!
//! There is no login. Anyone who can reach the address can read and edit the
//! projects, so keep `bind` on localhost or a private network/VPN.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

use clap::Parser;

use chickenscratch_web::config::{self, Overrides, Settings};
use chickenscratch_web::static_ui::UiSource;
use chickenscratch_web::{build_router, AppState};

#[derive(Parser, Debug)]
#[command(
    name = "chikn-web",
    version,
    about = "Self-hosted web shell for ChickenScratch"
)]
struct Cli {
    /// Folder that holds your .chikn projects. The server never touches
    /// project files outside it. Falls back to CHIKN_WEB_ROOT, then to
    /// `root` in the settings file.
    #[arg(long)]
    root: Option<PathBuf>,

    /// Address to listen on. Falls back to CHIKN_WEB_BIND, then to `bind`
    /// in the settings file, then to localhost (127.0.0.1:7391). Use
    /// 0.0.0.0:7391 to let other machines on your network or VPN reach it.
    #[arg(long)]
    bind: Option<SocketAddr>,

    /// Settings file to read instead of ~/.config/chickenscratch/web.toml.
    #[arg(long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Print the settings that would be used, with where each came from,
    /// and exit.
    #[arg(long)]
    show_config: bool,

    /// Folder with the built UI (index.html + assets/). Defaults to `ui_dir`
    /// in the settings file, then to the assets embedded in this binary when
    /// it was built with `--features embed-ui`, otherwise to `ui/dist` in the
    /// current directory.
    #[arg(long)]
    ui_dir: Option<PathBuf>,
}

/// Read the settings file (explicit `--config` or the default location) and
/// merge it with flags and environment. Errors are already plain sentences.
fn load_settings(cli: &Cli) -> Result<(Settings, PathBuf, bool), String> {
    let (file_path, explicit) = match &cli.config {
        Some(path) => (path.clone(), true),
        None => (
            config::default_path().unwrap_or_else(|| PathBuf::from(config::FILE_NAME)),
            false,
        ),
    };
    let file = config::load_file(&file_path, explicit).map_err(|e| e.to_string())?;
    let overrides = Overrides {
        root_flag: cli.root.clone(),
        root_env: std::env::var(config::ENV_ROOT).ok(),
        bind_flag: cli.bind,
        bind_env: std::env::var(config::ENV_BIND).ok(),
        ui_dir_flag: cli.ui_dir.clone(),
    };
    let settings =
        config::resolve(&overrides, file.as_ref(), &file_path).map_err(|e| e.to_string())?;
    Ok((settings, file_path, file.is_some()))
}

fn ui_dir_source(dir: PathBuf) -> Result<UiSource, String> {
    if dir.join("index.html").is_file() {
        Ok(UiSource::Dir(dir))
    } else {
        Err(format!(
            "UI build not found at {} — run `cd ui && npm run build`, pass --ui-dir, \
or build chikn-web with --features embed-ui",
            dir.display()
        ))
    }
}

fn resolve_ui(flag: Option<PathBuf>) -> Result<UiSource, String> {
    if let Some(dir) = flag {
        return ui_dir_source(dir);
    }
    #[cfg(feature = "embed-ui")]
    {
        Ok(UiSource::Embedded)
    }
    #[cfg(not(feature = "embed-ui"))]
    {
        ui_dir_source(PathBuf::from("ui/dist"))
    }
}

#[tokio::main]
async fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,tower_http=info".into()),
        )
        .init();

    let cli = Cli::parse();

    let (settings, file_path, file_found) = match load_settings(&cli) {
        Ok(s) => s,
        Err(msg) => {
            eprintln!("{msg}");
            return ExitCode::from(2);
        }
    };
    if cli.show_config {
        print!("{}", settings.describe(&file_path, file_found));
        return ExitCode::SUCCESS;
    }
    let Some(root) = settings.root.clone() else {
        eprintln!("{}", Settings::missing_root_message(&file_path));
        return ExitCode::from(2);
    };
    let bind = settings.bind;

    let ui = match resolve_ui(settings.ui_dir.clone()) {
        Ok(ui) => ui,
        Err(msg) => {
            eprintln!("{msg}");
            return ExitCode::from(2);
        }
    };

    let state = match AppState::new(&root, ui) {
        Ok(s) => Arc::new(s),
        Err(e) => {
            eprintln!("projects root {}: {e}", root.display());
            return ExitCode::from(2);
        }
    };

    let listener = match tokio::net::TcpListener::bind(bind).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("cannot listen on {}: {e}", bind);
            return ExitCode::from(1);
        }
    };
    let addr = listener.local_addr().unwrap_or(bind);

    tracing::info!(
        bind = %addr,
        root = %state.root.display(),
        root_source = %settings.root_source,
        settings_file = %file_path.display(),
        ui = %state.ui,
        "chikn-web listening"
    );
    println!("Open: http://{addr}/");

    let app = build_router(state);
    if let Err(e) = axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
    {
        eprintln!("server error: {e}");
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}

async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let terminate = async {
        if let Ok(mut sig) =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            sig.recv().await;
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    tracing::info!("shutting down");
}
