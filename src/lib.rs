//! Shared library for the Paperfoot accounting suite.
//!
//! Every finance-* CLI (invoice, receipt, expense, bank, recon, ledger, tax)
//! depends on this crate. It owns:
//!
//! - [`money`]: precise numeric handling (minor units + rust_decimal math)
//! - [`tax`]: jurisdiction tax profiles (SG GST, UK VAT, EU VAT, US, custom)
//!
//! Later phases will add: entity (shared company/issuer), settings (shared
//! config file), db (shared SQLite pool + refinery migrations), journal
//! (double-entry primitives), attachment (content-addressed blob store).

pub mod money;
pub mod tax;
