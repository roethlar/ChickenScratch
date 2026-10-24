//! `chikn-web` — run the ChickenScratch UI as a self-hosted web app.
//!
//! ```text
//! chikn-web --root ~/Notes                       # localhost only
//! chikn-web --root ~/Notes --bind 0.0.0.0:7391   # reachable over your VPN
//! ```
//!
//! On first run an access token is generated, stored under the user's config
//! directory with owner-only permissions, and printed once together with a
//! link that carries it. The UI stores the token in the browser after that.

use std::fs;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;

use clap::Parser;

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
    /// project files outside it.
    #[arg(long)]
    root: PathBuf,

    /// Address to listen on. Keep the default (localhost) unless you want
    /// other machines on your network or VPN to reach it.
    #[arg(long, default_value = "127.0.0.1:7391")]
    bind: SocketAddr,

    /// Access token. If omitted, read from CHIKN_WEB_TOKEN, then from the
    /// token file in the config directory, else generated and stored there.
    #[arg(long, env = "CHIKN_WEB_TOKEN")]
    token: Option<String>,

    /// Folder with the built UI (index.html + assets/). Defaults to the assets
    /// embedded in this binary when it was built with `--features embed-ui`,
    /// otherwise to `ui/dist` in the current directory.
    #[arg(long)]
    ui_dir: Option<PathBuf>,
}

enum TokenSource {
    Flag,
    File,
    Generated,
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

fn token_file() -> Option<PathBuf> {
    dirs::config_dir().map(|dir| dir.join("chickenscratch").join("web-token"))
}

fn generate_token() -> String {
    // Two v4 UUIDs: 244 bits of randomness, hex, no separators.
    format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )
}

fn write_token_file(path: &Path, token: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(path)?;
        file.write_all(token.as_bytes())?;
        file.write_all(b"\n")?;
        Ok(())
    }
    #[cfg(not(unix))]
    {
        fs::write(path, format!("{token}\n"))
    }
}

fn resolve_token(flag: Option<String>) -> std::io::Result<(String, TokenSource)> {
    if let Some(token) = flag.map(|t| t.trim().to_string()).filter(|t| !t.is_empty()) {
        return Ok((token, TokenSource::Flag));
    }
    let Some(path) = token_file() else {
        return Ok((generate_token(), TokenSource::Generated));
    };
    if let Ok(existing) = fs::read_to_string(&path) {
        let existing = existing.trim().to_string();
        if !existing.is_empty() {
            return Ok((existing, TokenSource::File));
        }
    }
    let token = generate_token();
    write_token_file(&path, &token)?;
    Ok((token, TokenSource::Generated))
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

    let ui = match resolve_ui(cli.ui_dir) {
        Ok(ui) => ui,
        Err(msg) => {
            eprintln!("{msg}");
            return ExitCode::from(2);
        }
    };

    let (token, source) = match resolve_token(cli.token) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("could not read or store the access token: {e}");
            return ExitCode::from(2);
        }
    };

    let state = match AppState::new(&cli.root, token.clone(), ui) {
        Ok(s) => Arc::new(s),
        Err(e) => {
            eprintln!("projects root {}: {e}", cli.root.display());
            return ExitCode::from(2);
        }
    };

    let listener = match tokio::net::TcpListener::bind(cli.bind).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("cannot listen on {}: {e}", cli.bind);
            return ExitCode::from(1);
        }
    };
    let addr = listener.local_addr().unwrap_or(cli.bind);

    tracing::info!(
        bind = %addr,
        root = %state.root.display(),
        ui = %state.ui,
        token_source = match source {
            TokenSource::Flag => "flag or CHIKN_WEB_TOKEN",
            TokenSource::File => "token file",
            TokenSource::Generated => "generated",
        },
        "chikn-web listening"
    );
    if matches!(source, TokenSource::Generated) {
        println!("Access token: {token}");
        if let Some(path) = token_file() {
            println!("(stored in {})", path.display());
        }
    }
    println!("Open: http://{addr}/?token={token}");

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
