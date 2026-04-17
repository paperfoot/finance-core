// ═══════════════════════════════════════════════════════════════════════════
// Settings — shared user preferences for the accounting suite.
//
// One config.toml at `Paths::config_file()` is read by every tool. Shared
// fields like `default_issuer` let `invoice new`, `receipt add`, and
// `ledger post` all default to the same company without reconfiguring.
// Tool-specific fields (default_template for invoice PDFs, etc.) live at
// the top level and are simply ignored by tools that don't care about them —
// TOML's "unknown key" tolerance via serde is perfect for that.
// ═══════════════════════════════════════════════════════════════════════════

use std::path::Path;

use figment::{
    providers::{Env, Format, Serialized, Toml},
    Figment,
};
use serde::{Deserialize, Serialize};

use crate::error::{CoreError, Result};
use crate::paths::Paths;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// Slug of the issuer (your company) to default to across the suite.
    /// `invoice new` uses it when --as is omitted; future tools honour it too.
    #[serde(default)]
    pub default_issuer: Option<String>,

    /// Default PDF / output template name. Honoured by tools that produce
    /// rendered artifacts (invoice-cli, future receipt-cli).
    #[serde(default = "default_template")]
    pub default_template: String,

    /// Whether to auto-open produced PDFs in the system viewer after render.
    #[serde(default = "default_true")]
    pub open_pdf: bool,

    /// Whether self-updating is enabled. Each tool's `update` subcommand
    /// respects this.
    #[serde(default = "default_true")]
    pub self_update: bool,
}

fn default_template() -> String {
    "vienna".to_string()
}

fn default_true() -> bool {
    true
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            default_issuer: None,
            default_template: default_template(),
            open_pdf: true,
            self_update: true,
        }
    }
}

impl Settings {
    /// Load settings from the shared config file + environment overrides.
    /// Missing file → returns `Default` with env applied. Env prefix is
    /// `PAPERFOOT_` (e.g. `PAPERFOOT_DEFAULT_ISSUER=acme`).
    pub fn load(paths: &Paths) -> Result<Self> {
        Self::load_from(&paths.config_file())
    }

    pub fn load_from(path: &Path) -> Result<Self> {
        let settings: Settings = Figment::from(Serialized::defaults(Settings::default()))
            .merge(Toml::file(path))
            .merge(Env::prefixed("PAPERFOOT_"))
            .extract()
            .map_err(|e| CoreError::Config(format!("{e}")))?;
        Ok(settings)
    }

    /// Persist settings to the shared config file.
    pub fn save(&self, paths: &Paths) -> Result<()> {
        self.save_to(&paths.config_file())
    }

    pub fn save_to(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let serialized = toml::to_string_pretty(self)?;
        std::fs::write(path, serialized)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn defaults_when_missing() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("config.toml");
        let s = Settings::load_from(&path).unwrap();
        assert_eq!(s.default_template, "vienna");
        assert!(s.open_pdf);
        assert!(s.self_update);
        assert!(s.default_issuer.is_none());
    }

    #[test]
    fn roundtrip_through_toml() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("config.toml");
        let written = Settings {
            default_issuer: Some("acme".into()),
            default_template: "boutique".into(),
            open_pdf: false,
            self_update: true,
        };
        written.save_to(&path).unwrap();
        let read = Settings::load_from(&path).unwrap();
        assert_eq!(read.default_issuer.as_deref(), Some("acme"));
        assert_eq!(read.default_template, "boutique");
        assert!(!read.open_pdf);
        assert!(read.self_update);
    }
}
