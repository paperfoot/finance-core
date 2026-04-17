-- ═══════════════════════════════════════════════════════════════════════════
-- V4 — issuer logos, discounts (line + invoice), credit notes
-- ═══════════════════════════════════════════════════════════════════════════

-- Logo per issuer. Absolute or ~/-relative path to a PNG/SVG/JPG file.
-- When set, templates render it in the header/accent band.
ALTER TABLE issuers ADD COLUMN logo_path TEXT;

-- Discounts on line items. Only one of (rate, fixed) should be set — both
-- reduce the line's post-qty amount pre-tax.
--   discount_rate:  decimal string ("10" = 10% off the line), tax_rate style
--   discount_fixed_minor: absolute discount in the invoice's minor units
ALTER TABLE invoice_items ADD COLUMN discount_rate        TEXT;
ALTER TABLE invoice_items ADD COLUMN discount_fixed_minor INTEGER;

-- Invoice-level discount: applied to the pre-tax subtotal after line-level
-- discounts. Again at most one of (rate, fixed) should be set.
ALTER TABLE invoices ADD COLUMN discount_rate        TEXT;
ALTER TABLE invoices ADD COLUMN discount_fixed_minor INTEGER;

-- Credit notes. Stored in the invoices table with kind='credit_note' and a
-- foreign-key-ish link back to the source invoice. The invoice number
-- generator gives credit notes their own sequential series per (issuer,year),
-- rendered with a "CN-" prefix.
ALTER TABLE invoices ADD COLUMN kind TEXT NOT NULL DEFAULT 'invoice';
ALTER TABLE invoices ADD COLUMN credits_invoice_id INTEGER;

-- number_series was keyed by (issuer_id, year) — now needs a third dimension
-- (kind) so credit-notes don't collide with invoice numbering.
CREATE TABLE number_series_new (
    issuer_id INTEGER NOT NULL,
    year      INTEGER NOT NULL,
    kind      TEXT    NOT NULL DEFAULT 'invoice',
    next_seq  INTEGER NOT NULL DEFAULT 1,
    PRIMARY KEY (issuer_id, year, kind),
    FOREIGN KEY (issuer_id) REFERENCES issuers(id) ON DELETE CASCADE
);
INSERT INTO number_series_new (issuer_id, year, kind, next_seq)
    SELECT issuer_id, year, 'invoice', next_seq FROM number_series;
DROP TABLE number_series;
ALTER TABLE number_series_new RENAME TO number_series;

CREATE INDEX idx_invoices_kind ON invoices(kind);
CREATE INDEX idx_invoices_credits ON invoices(credits_invoice_id);
