// ═══════════════════════════════════════════════════════════════════════════
// Paths — canonical locations for the Paperfoot accounting suite.
//
// Every finance-* CLI reads/writes state under one shared location so a
// user can switch between invoice-cli / receipt-cli / ledger-cli and see the
// same issuers, the same SQLite database, and the same config.toml.
//
// macOS:   ~/Library/Application Support/com.paperfoot.accounting/
// Linux:   ~/.local/share/com.paperfoot.accounting/  (+ ~/.config/ for config)
// Windows: %APPDATA%\paperfoot\accounting\
// ═══════════════════════════════════════════════════════════════════════════

use std::path::PathBuf;

use directories::ProjectDirs;

use crate::error::{CoreError, Result};

/// Canonical paths for the accounting suite. Every shared CLI should resolve
/// paths through this struct — never hardcode paths per-tool.
#[derive(Debug, Clone)]
pub struct Paths {
    pub config_dir: PathBuf,
    pub data_dir: PathBuf,
}

impl Paths {
    /// Resolve and create (if missing) the shared accounting-suite paths.
    pub fn resolve() -> Result<Self> {
        let dirs = ProjectDirs::from("com", "paperfoot", "accounting")
            .ok_or_else(|| CoreError::Path("could not resolve platform directories".into()))?;
        let config_dir = dirs.config_dir().to_path_buf();
        let data_dir = dirs.data_local_dir().to_path_buf();
        std::fs::create_dir_all(&config_dir)?;
        std::fs::create_dir_all(&data_dir)?;
        Ok(Self {
            config_dir,
            data_dir,
        })
    }

    pub fn config_file(&self) -> PathBuf {
        self.config_dir.join("config.toml")
    }

    pub fn db_file(&self) -> PathBuf {
        self.data_dir.join("accounting.db")
    }

    /// Directory for per-tool asset overrides (user-placed custom Typst
    /// templates, receipt image store root, etc). Each tool decides its own
    /// subdirectory name.
    pub fn assets_dir(&self) -> PathBuf {
        self.data_dir.join("assets")
    }
}
