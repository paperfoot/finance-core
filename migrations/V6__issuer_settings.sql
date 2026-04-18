-- ═══════════════════════════════════════════════════════════════════════════
-- V6 — per-issuer settings: default PDF output dir + default notes
--
-- Every company the user bills AS can now carry its own workflow defaults:
--   - default_output_dir: where `invoices render` drops PDFs when --out is
--     omitted. `~/` is expanded at read time. Example:
--       invoice issuers edit paperfoot --output-dir "~/Documents/Invoices/Paperfoot"
--   - default_notes: boilerplate text that `invoices new` uses unless the
--     user passes --notes explicitly. Ideal for payment terms, reverse
--     charge notices, bank-fee disclaimers, etc.
--
-- These are just additive TEXT columns — no data migration needed.
-- ═══════════════════════════════════════════════════════════════════════════

ALTER TABLE issuers ADD COLUMN default_output_dir TEXT;
ALTER TABLE issuers ADD COLUMN default_notes TEXT;
