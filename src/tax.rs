// ═══════════════════════════════════════════════════════════════════════════
// Tax profiles — jurisdiction-specific invoice behaviour.
//
// Each profile knows:
//   - tax label (GST / VAT / Sales tax / …)
//   - default rate
//   - currency + symbol
//   - whether "Tax Invoice" title is required when registered
//   - label for the registration number ("GST Reg. No." / "VAT No." / …)
//   - date format convention
//   - whether reverse-charge applies for cross-border B2B
// ═══════════════════════════════════════════════════════════════════════════

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Jurisdiction {
    Sg,
    Uk,
    Us,
    Eu,
    Custom,
}

impl Jurisdiction {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "sg" | "singapore" => Some(Self::Sg),
            "uk" | "gb" | "gbr" | "united-kingdom" => Some(Self::Uk),
            "us" | "usa" | "united-states" => Some(Self::Us),
            "eu" | "de" | "fr" | "nl" | "at" | "ie" => Some(Self::Eu),
            "custom" | "intl" | "international" => Some(Self::Custom),
            _ => None,
        }
    }

    pub fn profile(&self) -> TaxProfile {
        match self {
            Self::Sg => TaxProfile {
                code: "sg",
                country: "Singapore",
                tax_label: "GST",
                default_rate: 9.0,
                currency: "SGD",
                symbol: "S$",
                tax_invoice_title: "Tax Invoice",
                non_registered_title: "Invoice",
                tax_id_label: "GST Reg. No.",
                company_no_label: "UEN",
                date_format: "%-d %B %Y",
                supports_reverse_charge: false,
                zero_rate_label: "Zero-rated",
            },
            Self::Uk => TaxProfile {
                code: "uk",
                country: "United Kingdom",
                tax_label: "VAT",
                default_rate: 20.0,
                currency: "GBP",
                symbol: "£",
                tax_invoice_title: "VAT Invoice",
                non_registered_title: "Invoice",
                tax_id_label: "VAT No.",
                company_no_label: "Company No.",
                date_format: "%-d %B %Y",
                supports_reverse_charge: true,
                zero_rate_label: "Zero-rated",
            },
            Self::Us => TaxProfile {
                code: "us",
                country: "United States",
                tax_label: "Sales tax",
                default_rate: 0.0,
                currency: "USD",
                symbol: "$",
                tax_invoice_title: "Invoice",
                non_registered_title: "Invoice",
                tax_id_label: "EIN",
                company_no_label: "State ID",
                date_format: "%B %-d, %Y",
                supports_reverse_charge: false,
                zero_rate_label: "Exempt",
            },
            Self::Eu => TaxProfile {
                code: "eu",
                country: "European Union",
                tax_label: "VAT",
                default_rate: 19.0, // Germany default; users override
                currency: "EUR",
                symbol: "€",
                tax_invoice_title: "Rechnung / Invoice",
                non_registered_title: "Invoice",
                tax_id_label: "VAT ID",
                company_no_label: "Reg. No.",
                date_format: "%-d %B %Y",
                supports_reverse_charge: true,
                zero_rate_label: "Reverse charge",
            },
            Self::Custom => TaxProfile {
                code: "custom",
                country: "International",
                tax_label: "Tax",
                default_rate: 0.0,
                currency: "USD",
                symbol: "$",
                tax_invoice_title: "Invoice",
                non_registered_title: "Invoice",
                tax_id_label: "Tax ID",
                company_no_label: "Reg. No.",
                date_format: "%Y-%m-%d",
                supports_reverse_charge: false,
                zero_rate_label: "Zero-rated",
            },
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Sg => "sg",
            Self::Uk => "uk",
            Self::Us => "us",
            Self::Eu => "eu",
            Self::Custom => "custom",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TaxProfile {
    pub code: &'static str,
    pub country: &'static str,
    pub tax_label: &'static str,
    pub default_rate: f64,
    pub currency: &'static str,
    pub symbol: &'static str,
    pub tax_invoice_title: &'static str,
    pub non_registered_title: &'static str,
    pub tax_id_label: &'static str,
    pub company_no_label: &'static str,
    pub date_format: &'static str,
    pub supports_reverse_charge: bool,
    pub zero_rate_label: &'static str,
}

impl TaxProfile {
    pub fn title(&self, tax_registered: bool) -> &'static str {
        if tax_registered {
            self.tax_invoice_title
        } else {
            self.non_registered_title
        }
    }
}

pub fn all_profiles() -> Vec<TaxProfile> {
    vec![
        Jurisdiction::Sg.profile(),
        Jurisdiction::Uk.profile(),
        Jurisdiction::Us.profile(),
        Jurisdiction::Eu.profile(),
        Jurisdiction::Custom.profile(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sg_gst_defaults() {
        let p = Jurisdiction::Sg.profile();
        assert_eq!(p.tax_label, "GST");
        assert_eq!(p.default_rate, 9.0);
        assert_eq!(p.currency, "SGD");
        assert_eq!(p.title(true), "Tax Invoice");
        assert_eq!(p.title(false), "Invoice");
    }

    #[test]
    fn uk_vat_defaults() {
        let p = Jurisdiction::Uk.profile();
        assert_eq!(p.tax_label, "VAT");
        assert_eq!(p.default_rate, 20.0);
        assert_eq!(p.currency, "GBP");
        assert_eq!(p.title(true), "VAT Invoice");
    }

    #[test]
    fn parses_aliases() {
        assert_eq!(Jurisdiction::from_str("SG"), Some(Jurisdiction::Sg));
        assert_eq!(Jurisdiction::from_str("united-kingdom"), Some(Jurisdiction::Uk));
        assert_eq!(Jurisdiction::from_str("gb"), Some(Jurisdiction::Uk));
        assert_eq!(Jurisdiction::from_str("unknown"), None);
    }
}
