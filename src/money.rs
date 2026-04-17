// ═══════════════════════════════════════════════════════════════════════════
// Money — precise numeric handling.
//
// Amounts are stored as i64 minor units (cents). Tax math uses rust_decimal
// to avoid float rounding artefacts seen in some upstream Typst templates.
// ═══════════════════════════════════════════════════════════════════════════

use rust_decimal::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MinorUnits(pub i64);

impl MinorUnits {
    pub fn from_major(major: f64) -> Self {
        Self((major * 100.0).round() as i64)
    }

    pub fn from_decimal(d: Decimal) -> Self {
        let scaled = (d * Decimal::from(100)).round();
        Self(scaled.to_i64().unwrap_or(0))
    }

    pub fn as_major(&self) -> f64 {
        self.0 as f64 / 100.0
    }

    pub fn as_decimal(&self) -> Decimal {
        Decimal::from(self.0) / Decimal::from(100)
    }

    /// Format like `1,234.56` (no currency symbol).
    pub fn format_number(&self) -> String {
        let sign = if self.0 < 0 { "-" } else { "" };
        let abs = self.0.abs();
        let whole = abs / 100;
        let frac = abs % 100;
        let whole_str = format_thousands(whole);
        format!("{}{}.{:02}", sign, whole_str, frac)
    }

    /// Format with currency symbol: `S$1,234.56`.
    pub fn format_with_symbol(&self, symbol: &str) -> String {
        let sign = if self.0 < 0 { "-" } else { "" };
        let abs = Self(self.0.abs());
        format!("{}{}{}", sign, symbol, abs.format_number())
    }
}

fn format_thousands(n: i64) -> String {
    let s = n.to_string();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len();
    for (i, c) in chars.iter().enumerate() {
        out.push(*c);
        let remaining = len - i - 1;
        if remaining > 0 && remaining % 3 == 0 {
            out.push(',');
        }
    }
    out
}

/// Compute line total in minor units: qty * unit_price.
pub fn line_total(qty: Decimal, unit_price: MinorUnits) -> MinorUnits {
    let up = unit_price.as_decimal();
    MinorUnits::from_decimal(qty * up)
}

/// Apply a percent rate to a base. Shared by tax_amount and discount math.
pub fn apply_rate(base: MinorUnits, rate: Decimal) -> MinorUnits {
    let amt = base.as_decimal() * rate / Decimal::from(100);
    MinorUnits::from_decimal(amt)
}

/// Compute tax amount in minor units: base * rate / 100.
pub fn tax_amount(base: MinorUnits, rate: Decimal) -> MinorUnits {
    apply_rate(base, rate)
}

/// Line total after applying at most one of (rate discount, fixed discount).
/// If both are set, `rate` wins — caller should enforce mutual exclusion at
/// the CLI layer. Result is clamped at zero so a mis-sized fixed discount
/// can't flip the line negative.
pub fn line_total_discounted(
    qty: Decimal,
    unit_price: MinorUnits,
    discount_rate: Option<Decimal>,
    discount_fixed: Option<MinorUnits>,
) -> MinorUnits {
    let base = line_total(qty, unit_price);
    if let Some(rate) = discount_rate {
        let cut = apply_rate(base, rate);
        let res = base.0 - cut.0;
        return MinorUnits(res.max(0));
    }
    if let Some(fx) = discount_fixed {
        let res = base.0 - fx.0;
        return MinorUnits(res.max(0));
    }
    base
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn formats_thousands() {
        assert_eq!(MinorUnits(123456).format_number(), "1,234.56");
        assert_eq!(MinorUnits(100).format_number(), "1.00");
        assert_eq!(MinorUnits(99999999).format_number(), "999,999.99");
    }

    #[test]
    fn negative_formatted() {
        assert_eq!(MinorUnits(-12345).format_number(), "-123.45");
    }

    #[test]
    fn line_total_exact() {
        // 18.5 × 220.00 = 4070.00 exactly
        let total = line_total(dec!(18.5), MinorUnits::from_major(220.0));
        assert_eq!(total, MinorUnits::from_major(4070.0));
    }

    #[test]
    fn tax_exact() {
        // 24,600.00 × 9% = 2214.00
        let tax = tax_amount(MinorUnits::from_major(24600.0), dec!(9.0));
        assert_eq!(tax, MinorUnits::from_major(2214.0));
    }

    #[test]
    fn line_discount_rate() {
        // 10 × 100 = 1000, 10% off → 900
        let r = line_total_discounted(dec!(10), MinorUnits::from_major(100.0), Some(dec!(10)), None);
        assert_eq!(r, MinorUnits::from_major(900.0));
    }

    #[test]
    fn line_discount_fixed() {
        // 1 × 500 = 500, fixed 50 off → 450
        let r = line_total_discounted(
            dec!(1),
            MinorUnits::from_major(500.0),
            None,
            Some(MinorUnits::from_major(50.0)),
        );
        assert_eq!(r, MinorUnits::from_major(450.0));
    }

    #[test]
    fn line_discount_clamps_at_zero() {
        // Over-discount — shouldn't go negative
        let r = line_total_discounted(
            dec!(1),
            MinorUnits::from_major(10.0),
            None,
            Some(MinorUnits::from_major(999.0)),
        );
        assert_eq!(r, MinorUnits(0));
    }
}
