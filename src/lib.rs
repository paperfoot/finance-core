//! Shared library for the Paperfoot accounting suite.
//!
//! Every finance-* CLI (invoice, receipt, expense, bank, recon, ledger, tax)
//! depends on this crate. It owns:
//!
//! - [`money`]: precise numeric handling (minor units + rust_decimal math)
//! - [`tax`]: jurisdiction tax profiles (SG GST, UK VAT, EU VAT, US, custom)
//! - [`entity`]: the `Issuer` primitive (companies you transact as)
//! - [`error`]: shared `CoreError` with thiserror variants
//! - [`paths`]: canonical suite paths (`~/.../com.paperfoot.accounting/`)
//! - [`settings`]: the shared `config.toml` reader/writer
//! - [`db`]: the shared SQLite connection + refinery migration runner
//!
//! The intent is that every new CLI in the suite starts by calling
//! `let paths = Paths::resolve()?; let conn = db::open(&paths)?;` and gets a
//! fully-migrated database ready to use.

pub mod db;
pub mod entity;
pub mod error;
pub mod money;
pub mod paths;
pub mod settings;
pub mod tax;
