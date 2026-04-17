-- ═══════════════════════════════════════════════════════════════════════════
-- V3 — per-client defaults + invoice lifecycle timestamps
-- ═══════════════════════════════════════════════════════════════════════════

-- Each client can now pin a default issuer (the company they're always
-- invoiced from) and/or a default template (their preferred visual style).
-- Precedence at invoice creation / render:
--   --as CLI arg        >  client.default_issuer_slug  (for `invoices new`)
--   --template CLI arg  >  client.default_template   >  issuer.default_template
ALTER TABLE clients ADD COLUMN default_issuer_slug TEXT;
ALTER TABLE clients ADD COLUMN default_template   TEXT;

-- Lifecycle audit: issued_at is stamped the first time an invoice transitions
-- to 'issued'. paid_at is stamped the first time it transitions to 'paid'.
-- Enables accounting queries like "what did we send in Q1?" and ageing.
ALTER TABLE invoices ADD COLUMN issued_at TEXT;
ALTER TABLE invoices ADD COLUMN paid_at   TEXT;
