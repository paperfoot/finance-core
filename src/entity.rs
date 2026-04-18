// ═══════════════════════════════════════════════════════════════════════════
// Entity — the canonical "company" primitive shared across the suite.
//
// Historical name is `Issuer` (a company that ISSUES invoices); we keep the
// type name `Issuer` for now since invoice-cli is the only consumer and
// renaming every call-site adds churn for no gain. Later tools (receipt-cli
// for merchants, ledger-cli for counterparties) will query the same table.
//
// `bank_details` is a free-form multi-line string with one "Label: Value" per
// line. This handles every country without per-jurisdiction schema (SG bank
// code / UK sort code / US ABA routing / EU IBAN / AU BSB …).
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
    /// Free-form multi-line bank / payment details. One `Label: Value` per
    /// line. Renderer splits on the first ":" per line and displays a
    /// two-column table on the PDF. Example:
    ///
    /// ```text
    /// Bank: Standard Chartered Bank (Singapore) Ltd
    /// Account: 7897262250
    /// Bank Code: 9496
    /// Branch Code: 001
    /// SWIFT: SCBLSG22
    /// ```
    pub bank_details: Option<String>,
    pub default_template: String,
    pub currency: Option<String>,
    pub symbol: Option<String>,
    pub number_format: String,
    /// Filesystem path to a logo image (PNG/SVG/JPG). Rendered in template
    /// header when set.
    pub logo_path: Option<String>,
}

/// One row of the rendered payment block. Parsed from `Issuer::bank_details`
/// by splitting each non-empty line on the first ':'.
#[derive(Debug, Clone, Serialize)]
pub struct BankLine {
    pub label: String,
    pub value: String,
}

impl BankLine {
    /// Parse the multi-line `bank_details` field into labelled rows.
    /// Lines without a ':' get an empty label — the renderer treats them as
    /// continuation text.
    pub fn parse_all(details: &str) -> Vec<Self> {
        details
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(|line| match line.split_once(':') {
                Some((label, value)) => Self {
                    label: label.trim().to_string(),
                    value: value.trim().to_string(),
                },
                None => Self {
                    label: String::new(),
                    value: line.to_string(),
                },
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_sg_bank_details() {
        let input = "Bank: Standard Chartered Bank (Singapore) Ltd\n\
                     Account: 7897262250\n\
                     Bank Code: 9496\n\
                     Branch Code: 001\n\
                     SWIFT: SCBLSG22";
        let lines = BankLine::parse_all(input);
        assert_eq!(lines.len(), 5);
        assert_eq!(lines[0].label, "Bank");
        assert_eq!(lines[0].value, "Standard Chartered Bank (Singapore) Ltd");
        assert_eq!(lines[3].label, "Branch Code");
        assert_eq!(lines[3].value, "001");
    }

    #[test]
    fn handles_lines_without_colon() {
        let input = "Use FAST for SG transfers\nAccount: 7897262250";
        let lines = BankLine::parse_all(input);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].label, "");
        assert_eq!(lines[0].value, "Use FAST for SG transfers");
        assert_eq!(lines[1].label, "Account");
    }

    #[test]
    fn skips_blank_lines() {
        let input = "Bank: DBS\n\n\nAccount: 123";
        assert_eq!(BankLine::parse_all(input).len(), 2);
    }
}
