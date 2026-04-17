// ═══════════════════════════════════════════════════════════════════════════
// CoreError — errors surfaced by shared accounting-suite primitives.
//
// Consumer CLIs (invoice-cli, receipt-cli, …) typically define their own
// AppError with a `#[from] CoreError` variant so they can wrap and extend it.
// ═══════════════════════════════════════════════════════════════════════════

use thiserror::Error;

#[derive(Error, Debug)]
pub enum CoreError {
    #[error("config error: {0}")]
    Config(String),

    #[error("path resolution failed: {0}")]
    Path(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),

    #[error("migration error: {0}")]
    Migration(#[from] refinery::Error),

    #[error("serialization error: {0}")]
    Toml(#[from] toml::de::Error),

    #[error("serialization error: {0}")]
    TomlSer(#[from] toml::ser::Error),
}

pub type Result<T> = std::result::Result<T, CoreError>;
