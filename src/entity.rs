// ═══════════════════════════════════════════════════════════════════════════
// Entity — the canonical "company" primitive shared across the suite.
//
// Historical name is `Issuer` (a company that ISSUES invoices); we keep the
// type name `Issuer` for now since invoice-cli is the only consumer and
// renaming every call-site adds churn for no gain. Later tools (receipt-cli
// for merchants, ledger-cli for counterparties) will query the same table.
// ═══════════════════════════════════════════════════════════════════════════

use serde::{Deserialize, Serialize};

use crate::tax::Jurisdiction;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issuer {
    pub id: i64,
    pub slug: String,
    pub name: String,
    pub legal_name: Option<String>,
    pub jurisdiction: Jurisdiction,
    pub tax_registered: bool,
    pub tax_id: Option<String>,
    pub company_no: Option<String>,
    pub tagline: Option<String>,
    pub address: Vec<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub bank_name: Option<String>,
    pub bank_iban: Option<String>,
    pub bank_bic: Option<String>,
    pub default_template: String,
    pub currency: Option<String>,
    pub symbol: Option<String>,
    pub number_format: String,
    /// Filesystem path to a logo image (PNG/SVG/JPG). Rendered in template
    /// header when set.
    pub logo_path: Option<String>,
}
