//! The settings file: `~/.config/chickenscratch/web.toml`
//! (`docs/plans/PLAN_WEB_INSTALLER.md`, slice 1).
//!
//! ```toml
//! root = "/mnt/stuff/Notes"   # folder that holds the .chikn projects
//! bind = "0.0.0.0:7391"       # address:port to listen on
//! # backup = "/mnt/stuff/Backups"  # optional: a second folder the browser may pick from
//! # verify_repo_owner = true      # keep libgit2's folder-ownership check (off by default here)
//! # ui_dir = "/path/to/ui/dist"   # development builds only
//! ```
//!
//! Every value can also come from the command line or the environment.
//! Precedence, highest first: flag → environment (`CHIKN_WEB_ROOT`,
//! `CHIKN_WEB_BIND`, `CHIKN_WEB_BACKUP`) → settings file → built-in default.
//! `root` has no default; without one anywhere the server refuses to start
//! and names the file. `backup` is optional: when set, the folder picker in
//! the browser offers it as a second place to browse (for the backup
//! setting), outside the projects root.
//!
//! Paths in the file may start with `~/`, which means the home directory.
//! Any other relative path is taken relative to the working directory the
//! server was started in, exactly like a flag.

use std::fmt;
use std::fs;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use serde::Deserialize;

/// Name of the settings file inside the config directory.
pub const FILE_NAME: &str = "web.toml";
pub const ENV_ROOT: &str = "CHIKN_WEB_ROOT";
pub const ENV_BIND: &str = "CHIKN_WEB_BIND";
pub const ENV_BACKUP: &str = "CHIKN_WEB_BACKUP";
pub const DEFAULT_BIND: &str = "127.0.0.1:7391";

/// Exactly what the file may contain. Unknown keys are an error so a typo
/// such as `roots = …` is caught instead of silently ignored.
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileConfig {
    pub root: Option<PathBuf>,
    pub bind: Option<String>,
    pub backup: Option<PathBuf>,
    pub ui_dir: Option<PathBuf>,
    /// Keep libgit2's refusal of repositories owned by another user. Off by
    /// default on the server, where a NAS that maps ownership is the common
    /// case (`PLAN_WEB_SERVER_FIXES.md` slice 3).
    pub verify_repo_owner: Option<bool>,
}

/// Values given on the command line or found in the environment, before
/// the file is consulted. `main` fills this from clap and `std::env`.
#[derive(Debug, Default, Clone)]
pub struct Overrides {
    pub root_flag: Option<PathBuf>,
    pub root_env: Option<String>,
    pub bind_flag: Option<SocketAddr>,
    pub bind_env: Option<String>,
    pub backup_flag: Option<PathBuf>,
    pub backup_env: Option<String>,
    pub ui_dir_flag: Option<PathBuf>,
    /// `--verify-repo-owner` on the command line.
    pub verify_repo_owner_flag: bool,
}

/// Where a resolved value came from, for `--show-config`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    Flag,
    Env(&'static str),
    File(PathBuf),
    Default,
    Unset,
}

impl fmt::Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Source::Flag => f.write_str("command line"),
            Source::Env(name) => write!(f, "environment {name}"),
            Source::File(path) => write!(f, "{}", path.display()),
            Source::Default => f.write_str("built-in default"),
            Source::Unset => f.write_str("not set"),
        }
    }
}

/// The settings the server will run with, each tagged with its source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    pub root: Option<PathBuf>,
    pub root_source: Source,
    pub bind: SocketAddr,
    pub bind_source: Source,
    pub backup: Option<PathBuf>,
    pub backup_source: Source,
    pub ui_dir: Option<PathBuf>,
    pub ui_dir_source: Source,
    pub verify_repo_owner: bool,
    pub verify_repo_owner_source: Source,
}

impl Settings {
    /// The one-sentence refusal when no projects folder is set anywhere.
    pub fn missing_root_message(file: &Path) -> String {
        format!(
            "No projects folder set. Pass --root <folder>, set {ENV_ROOT}, \
or put root = \"/path/to/projects\" in {}.",
            file.display()
        )
    }

    /// Plain text for `--show-config`: one line per setting with its source.
    pub fn describe(&self, file: &Path, file_found: bool) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "settings file: {} ({})\n",
            file.display(),
            if file_found { "found" } else { "not present" }
        ));
        match &self.root {
            Some(root) => out.push_str(&format!(
                "root = {}  [{}]\n",
                root.display(),
                self.root_source
            )),
            None => out.push_str(&format!("root = (not set)  [{}]\n", self.root_source)),
        }
        out.push_str(&format!("bind = {}  [{}]\n", self.bind, self.bind_source));
        match &self.backup {
            Some(dir) => out.push_str(&format!(
                "backup = {}  [{}]\n",
                dir.display(),
                self.backup_source
            )),
            None => out.push_str(&format!("backup = (none)  [{}]\n", self.backup_source)),
        }
        match &self.ui_dir {
            Some(dir) => out.push_str(&format!(
                "ui_dir = {}  [{}]\n",
                dir.display(),
                self.ui_dir_source
            )),
            None => out.push_str(&format!(
                "ui_dir = (embedded UI)  [{}]\n",
                self.ui_dir_source
            )),
        }
        out.push_str(&format!(
            "verify_repo_owner = {}  [{}]\n",
            self.verify_repo_owner, self.verify_repo_owner_source
        ));
        out
    }
}

#[derive(Debug)]
pub enum ConfigError {
    /// An explicitly requested file (`--config`) does not exist.
    NotFound(PathBuf),
    Read(PathBuf, std::io::Error),
    /// Malformed TOML or an unknown key; the message carries the line.
    Parse(PathBuf, String),
    /// A `bind` value that is not `address:port`, with where it came from.
    Bind {
        value: String,
        source: Source,
    },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::NotFound(p) => {
                write!(f, "settings file {} does not exist", p.display())
            }
            ConfigError::Read(p, e) => write!(f, "cannot read {}: {e}", p.display()),
            ConfigError::Parse(p, msg) => write!(f, "{}: {msg}", p.display()),
            ConfigError::Bind { value, source } => write!(
                f,
                "bind = {value:?} ({source}) is not an address:port such as {DEFAULT_BIND}"
            ),
        }
    }
}

impl std::error::Error for ConfigError {}

/// `~/.config/chickenscratch/web.toml` on Linux (`$XDG_CONFIG_HOME` honoured),
/// the platform equivalent elsewhere. `None` only when the platform reports
/// no config directory at all.
pub fn default_path() -> Option<PathBuf> {
    dirs::config_dir().map(|dir| dir.join("chickenscratch").join(FILE_NAME))
}

/// Read and parse the file. `Ok(None)` when it does not exist and was not
/// explicitly requested; an explicit path must exist.
pub fn load_file(path: &Path, explicit: bool) -> Result<Option<FileConfig>, ConfigError> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return if explicit {
                Err(ConfigError::NotFound(path.to_path_buf()))
            } else {
                Ok(None)
            };
        }
        Err(e) => return Err(ConfigError::Read(path.to_path_buf(), e)),
    };
    parse(&text)
        .map(Some)
        .map_err(|msg| ConfigError::Parse(path.to_path_buf(), msg))
}

/// Parse the file's text; the error message names the line and column.
pub fn parse(text: &str) -> Result<FileConfig, String> {
    toml::from_str::<FileConfig>(text).map_err(|e| e.to_string().trim().to_string())
}

/// Combine flags, environment and file into the settings the server uses.
pub fn resolve(
    overrides: &Overrides,
    file: Option<&FileConfig>,
    file_path: &Path,
) -> Result<Settings, ConfigError> {
    let from_file = Source::File(file_path.to_path_buf());

    let (root, root_source) = if let Some(root) = &overrides.root_flag {
        (Some(root.clone()), Source::Flag)
    } else if let Some(root) = non_empty(overrides.root_env.as_deref()) {
        (Some(expand_home(Path::new(root))), Source::Env(ENV_ROOT))
    } else if let Some(root) = file.and_then(|f| f.root.as_deref()) {
        (Some(expand_home(root)), from_file.clone())
    } else {
        (None, Source::Unset)
    };

    let (bind, bind_source) = if let Some(bind) = overrides.bind_flag {
        (bind, Source::Flag)
    } else if let Some(bind) = non_empty(overrides.bind_env.as_deref()) {
        (
            parse_bind(bind, Source::Env(ENV_BIND))?,
            Source::Env(ENV_BIND),
        )
    } else if let Some(bind) = file.and_then(|f| f.bind.as_deref()) {
        (parse_bind(bind, from_file.clone())?, from_file.clone())
    } else {
        (
            DEFAULT_BIND.parse().expect("default bind parses"),
            Source::Default,
        )
    };

    let (backup, backup_source) = if let Some(dir) = &overrides.backup_flag {
        (Some(dir.clone()), Source::Flag)
    } else if let Some(dir) = non_empty(overrides.backup_env.as_deref()) {
        (Some(expand_home(Path::new(dir))), Source::Env(ENV_BACKUP))
    } else if let Some(dir) = file.and_then(|f| f.backup.as_deref()) {
        (Some(expand_home(dir)), from_file.clone())
    } else {
        (None, Source::Unset)
    };

    let (ui_dir, ui_dir_source) = if let Some(dir) = &overrides.ui_dir_flag {
        (Some(dir.clone()), Source::Flag)
    } else if let Some(dir) = file.and_then(|f| f.ui_dir.as_deref()) {
        (Some(expand_home(dir)), from_file.clone())
    } else {
        (None, Source::Default)
    };

    let (verify_repo_owner, verify_repo_owner_source) = if overrides.verify_repo_owner_flag {
        (true, Source::Flag)
    } else if let Some(v) = file.and_then(|f| f.verify_repo_owner) {
        (v, from_file)
    } else {
        (false, Source::Default)
    };

    Ok(Settings {
        root,
        root_source,
        bind,
        bind_source,
        backup,
        backup_source,
        ui_dir,
        ui_dir_source,
        verify_repo_owner,
        verify_repo_owner_source,
    })
}

fn non_empty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|v| !v.is_empty())
}

fn parse_bind(value: &str, source: Source) -> Result<SocketAddr, ConfigError> {
    value.trim().parse().map_err(|_| ConfigError::Bind {
        value: value.to_string(),
        source,
    })
}

/// `~` or `~/x` → the home directory; anything else unchanged.
pub fn expand_home(path: &Path) -> PathBuf {
    let Some(home) = dirs::home_dir() else {
        return path.to_path_buf();
    };
    let Ok(rest) = path.strip_prefix("~") else {
        return path.to_path_buf();
    };
    if rest.as_os_str().is_empty() {
        home
    } else {
        home.join(rest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(root: Option<&str>, bind: Option<&str>) -> FileConfig {
        FileConfig {
            root: root.map(PathBuf::from),
            bind: bind.map(str::to_string),
            backup: None,
            ui_dir: None,
            verify_repo_owner: None,
        }
    }

    fn path() -> PathBuf {
        PathBuf::from("/home/me/.config/chickenscratch/web.toml")
    }

    #[test]
    fn flag_alone_sets_root_and_defaults_bind() {
        let o = Overrides {
            root_flag: Some(PathBuf::from("/srv/notes")),
            ..Default::default()
        };
        let s = resolve(&o, None, &path()).unwrap();
        assert_eq!(s.root.as_deref(), Some(Path::new("/srv/notes")));
        assert_eq!(s.root_source, Source::Flag);
        assert_eq!(s.bind, DEFAULT_BIND.parse().unwrap());
        assert_eq!(s.bind_source, Source::Default);
        assert_eq!(s.ui_dir, None);
    }

    #[test]
    fn file_supplies_everything_when_nothing_else_does() {
        let f = file(Some("/mnt/stuff/Notes"), Some("0.0.0.0:7391"));
        let s = resolve(&Overrides::default(), Some(&f), &path()).unwrap();
        assert_eq!(s.root.as_deref(), Some(Path::new("/mnt/stuff/Notes")));
        assert_eq!(s.root_source, Source::File(path()));
        assert_eq!(s.bind, "0.0.0.0:7391".parse().unwrap());
        assert_eq!(s.bind_source, Source::File(path()));
    }

    #[test]
    fn flag_beats_env_beats_file() {
        let f = file(Some("/from/file"), Some("0.0.0.0:1"));
        let o = Overrides {
            root_flag: Some(PathBuf::from("/from/flag")),
            root_env: Some("/from/env".into()),
            bind_env: Some("0.0.0.0:2".into()),
            ..Default::default()
        };
        let s = resolve(&o, Some(&f), &path()).unwrap();
        assert_eq!(s.root.as_deref(), Some(Path::new("/from/flag")));
        assert_eq!(s.root_source, Source::Flag);
        assert_eq!(s.bind, "0.0.0.0:2".parse().unwrap());
        assert_eq!(s.bind_source, Source::Env(ENV_BIND));

        let o = Overrides {
            root_env: Some("/from/env".into()),
            bind_flag: Some("0.0.0.0:3".parse().unwrap()),
            ..Default::default()
        };
        let s = resolve(&o, Some(&f), &path()).unwrap();
        assert_eq!(s.root.as_deref(), Some(Path::new("/from/env")));
        assert_eq!(s.root_source, Source::Env(ENV_ROOT));
        assert_eq!(s.bind_source, Source::Flag);
    }

    #[test]
    fn empty_env_values_do_not_count() {
        let f = file(Some("/from/file"), None);
        let o = Overrides {
            root_env: Some("   ".into()),
            bind_env: Some(String::new()),
            ..Default::default()
        };
        let s = resolve(&o, Some(&f), &path()).unwrap();
        assert_eq!(s.root.as_deref(), Some(Path::new("/from/file")));
        assert_eq!(s.bind_source, Source::Default);
    }

    #[test]
    fn no_root_anywhere_is_reported_not_defaulted() {
        let s = resolve(&Overrides::default(), None, &path()).unwrap();
        assert_eq!(s.root, None);
        assert_eq!(s.root_source, Source::Unset);
        let msg = Settings::missing_root_message(&path());
        assert!(msg.contains("--root"));
        assert!(msg.contains(ENV_ROOT));
        assert!(msg.contains("web.toml"));
    }

    #[test]
    fn bad_bind_names_where_it_came_from() {
        let f = file(None, Some("not-an-address"));
        let err = resolve(&Overrides::default(), Some(&f), &path()).unwrap_err();
        let text = err.to_string();
        assert!(text.contains("not-an-address"), "{text}");
        assert!(text.contains("web.toml"), "{text}");

        let o = Overrides {
            bind_env: Some("8080".into()),
            ..Default::default()
        };
        let text = resolve(&o, None, &path()).unwrap_err().to_string();
        assert!(text.contains(ENV_BIND), "{text}");
    }

    #[test]
    fn parses_the_documented_file() {
        let f = parse(
            "root = \"/mnt/stuff/Notes\"   # folder that holds the .chikn projects\n\
             bind = \"0.0.0.0:7391\"\n",
        )
        .unwrap();
        assert_eq!(f, file(Some("/mnt/stuff/Notes"), Some("0.0.0.0:7391")));
        assert_eq!(parse("").unwrap(), FileConfig::default());
    }

    #[test]
    fn malformed_file_names_the_line() {
        let err = parse("root = \"/ok\"\nbind = 0.0.0.0:7391\n").unwrap_err();
        assert!(err.contains("line 2"), "{err}");
    }

    #[test]
    fn unknown_key_is_an_error() {
        let err = parse("roots = \"/typo\"\n").unwrap_err();
        assert!(err.contains("roots"), "{err}");
    }

    #[test]
    fn missing_file_is_fine_unless_explicitly_requested() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join(FILE_NAME);
        assert_eq!(load_file(&missing, false).unwrap(), None);
        assert!(matches!(
            load_file(&missing, true),
            Err(ConfigError::NotFound(_))
        ));

        fs::write(&missing, "root = \"~/Notes\"\n").unwrap();
        let loaded = load_file(&missing, false).unwrap().unwrap();
        assert_eq!(loaded.root.as_deref(), Some(Path::new("~/Notes")));
        let s = resolve(&Overrides::default(), Some(&loaded), &missing).unwrap();
        let home = dirs::home_dir().unwrap();
        assert_eq!(s.root, Some(home.join("Notes")));
    }

    #[test]
    fn backup_folder_is_optional_and_follows_the_same_precedence() {
        let s = resolve(&Overrides::default(), None, &path()).unwrap();
        assert_eq!(s.backup, None);
        assert_eq!(s.backup_source, Source::Unset);

        let f = parse("root = \"/n\"\nbackup = \"~/Backups\"\n").unwrap();
        let s = resolve(&Overrides::default(), Some(&f), &path()).unwrap();
        assert_eq!(s.backup, Some(dirs::home_dir().unwrap().join("Backups")));
        assert_eq!(s.backup_source, Source::File(path()));

        let o = Overrides {
            backup_env: Some("/env/backups".into()),
            ..Default::default()
        };
        let s = resolve(&o, Some(&f), &path()).unwrap();
        assert_eq!(s.backup.as_deref(), Some(Path::new("/env/backups")));
        assert_eq!(s.backup_source, Source::Env(ENV_BACKUP));
        assert!(s.describe(&path(), true).contains("backup = /env/backups"));
    }

    #[test]
    fn repository_ownership_check_is_off_unless_asked_for() {
        let s = resolve(&Overrides::default(), None, &path()).unwrap();
        assert!(!s.verify_repo_owner);
        assert_eq!(s.verify_repo_owner_source, Source::Default);

        let f = parse("root = \"/n\"\nverify_repo_owner = true\n").unwrap();
        let s = resolve(&Overrides::default(), Some(&f), &path()).unwrap();
        assert!(s.verify_repo_owner);
        assert_eq!(s.verify_repo_owner_source, Source::File(path()));

        let o = Overrides {
            verify_repo_owner_flag: true,
            ..Default::default()
        };
        let s = resolve(&o, None, &path()).unwrap();
        assert!(s.verify_repo_owner);
        assert!(s
            .describe(&path(), false)
            .contains("verify_repo_owner = true  [command line]"));
    }

    #[test]
    fn expand_home_only_touches_a_leading_tilde() {
        let home = dirs::home_dir().unwrap();
        assert_eq!(expand_home(Path::new("~")), home);
        assert_eq!(expand_home(Path::new("~/a/b")), home.join("a/b"));
        assert_eq!(expand_home(Path::new("/x/~/y")), PathBuf::from("/x/~/y"));
        assert_eq!(expand_home(Path::new("rel/~")), PathBuf::from("rel/~"));
    }

    #[test]
    fn describe_lists_every_setting_with_its_source() {
        let f = file(Some("/mnt/stuff/Notes"), None);
        let s = resolve(&Overrides::default(), Some(&f), &path()).unwrap();
        let text = s.describe(&path(), true);
        assert!(text.contains("root = /mnt/stuff/Notes"), "{text}");
        assert!(
            text.contains("bind = 127.0.0.1:7391  [built-in default]"),
            "{text}"
        );
        assert!(text.contains("ui_dir = (embedded UI)"), "{text}");
        assert!(text.contains("(found)"), "{text}");
    }
}
