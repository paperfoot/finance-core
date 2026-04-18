-- ═══════════════════════════════════════════════════════════════════════════
-- V5 — flexible bank details
--
-- Replaces the IBAN-centric trio (bank_name / bank_iban / bank_bic) with one
-- free-form `bank_details` column. One "Label: Value" per line. Suits every
-- country: SG (bank code + branch code + account), UK (sort code + account),
-- US (ABA routing), EU (IBAN + BIC), AU (BSB), anywhere. No per-jurisdiction
-- schema work needed.
--
-- The renderer splits each line on the first ":" and shows a two-column list
-- on the invoice PDF.
-- ═══════════════════════════════════════════════════════════════════════════

ALTER TABLE issuers ADD COLUMN bank_details TEXT;

-- Backfill any existing data into the new column using sensible labels.
UPDATE issuers
SET bank_details = NULLIF(
    TRIM(
        COALESCE('Bank: ' || bank_name || CHAR(10), '') ||
        COALESCE('Account: ' || bank_iban || CHAR(10), '') ||
        COALESCE('SWIFT: ' || bank_bic || CHAR(10), ''),
        CHAR(10)
    ),
    ''
)
WHERE bank_name IS NOT NULL OR bank_iban IS NOT NULL OR bank_bic IS NOT NULL;

-- Drop the legacy columns (requires SQLite 3.35+; rusqlite 0.33 bundles 3.45+).
ALTER TABLE issuers DROP COLUMN bank_name;
ALTER TABLE issuers DROP COLUMN bank_iban;
ALTER TABLE issuers DROP COLUMN bank_bic;
