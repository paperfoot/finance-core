-- ═══════════════════════════════════════════════════════════════════════════
-- invoice-cli — initial schema
--
-- Multi-issuer: every invoice belongs to an issuer (company/brand) which has
-- its own jurisdiction, tax profile, and numbering series. Lets one user run
-- several companies (SG entity + UK entity + …) from one CLI.
-- ═══════════════════════════════════════════════════════════════════════════

CREATE TABLE issuers (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    slug            TEXT NOT NULL UNIQUE,
    name            TEXT NOT NULL,           -- display name, e.g. "Acme"
    legal_name      TEXT,                    -- "Acme Studio Pte. Ltd."
    jurisdiction    TEXT NOT NULL,           -- sg / uk / us / eu / custom
    tax_registered  INTEGER NOT NULL DEFAULT 0,
    tax_id          TEXT,                    -- GST reg / VAT no / EIN
    company_no      TEXT,                    -- UEN / Companies House no.
    tagline         TEXT,
    address         TEXT NOT NULL,           -- newline-separated
    email           TEXT,
    phone           TEXT,
    bank_name       TEXT,
    bank_iban       TEXT,
    bank_bic        TEXT,
    default_template TEXT NOT NULL DEFAULT 'vienna',
    currency        TEXT,                    -- override jurisdiction default
    symbol          TEXT,
    number_format   TEXT NOT NULL DEFAULT '{year}-{seq:04}',
    created_at      TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE clients (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    slug          TEXT NOT NULL UNIQUE,
    name          TEXT NOT NULL,
    attn          TEXT,
    country       TEXT,
    tax_id        TEXT,
    address       TEXT NOT NULL,             -- newline-separated
    email         TEXT,
    notes         TEXT,
    created_at    TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE products (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    slug           TEXT NOT NULL UNIQUE,
    description    TEXT NOT NULL,
    subtitle       TEXT,
    unit           TEXT NOT NULL DEFAULT 'unit',
    unit_price_minor INTEGER NOT NULL,        -- stored in minor units
    currency       TEXT NOT NULL,
    tax_rate       TEXT NOT NULL DEFAULT '0', -- string for rust_decimal
    created_at     TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE number_series (
    issuer_id    INTEGER NOT NULL,
    year         INTEGER NOT NULL,
    next_seq     INTEGER NOT NULL DEFAULT 1,
    PRIMARY KEY (issuer_id, year),
    FOREIGN KEY (issuer_id) REFERENCES issuers(id) ON DELETE CASCADE
);

CREATE TABLE invoices (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    number          TEXT NOT NULL UNIQUE,
    issuer_id       INTEGER NOT NULL,
    client_id       INTEGER NOT NULL,
    issue_date      TEXT NOT NULL,            -- ISO 8601
    due_date        TEXT NOT NULL,
    terms           TEXT NOT NULL,
    currency        TEXT NOT NULL,
    symbol          TEXT NOT NULL,
    tax_label       TEXT NOT NULL,
    status          TEXT NOT NULL DEFAULT 'draft', -- draft / issued / paid / void
    notes           TEXT,
    reverse_charge  INTEGER NOT NULL DEFAULT 0,
    created_at      TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (issuer_id) REFERENCES issuers(id),
    FOREIGN KEY (client_id) REFERENCES clients(id)
);

CREATE TABLE invoice_items (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    invoice_id       INTEGER NOT NULL,
    position         INTEGER NOT NULL,         -- display order
    description      TEXT NOT NULL,
    subtitle         TEXT,
    qty              TEXT NOT NULL,            -- rust_decimal string
    unit             TEXT NOT NULL,
    unit_price_minor INTEGER NOT NULL,
    tax_rate         TEXT NOT NULL,
    product_id       INTEGER,                  -- optional source product
    FOREIGN KEY (invoice_id) REFERENCES invoices(id) ON DELETE CASCADE,
    FOREIGN KEY (product_id) REFERENCES products(id)
);

CREATE INDEX idx_invoices_issuer  ON invoices(issuer_id);
CREATE INDEX idx_invoices_client  ON invoices(client_id);
CREATE INDEX idx_invoices_status  ON invoices(status);
CREATE INDEX idx_invoices_issue   ON invoices(issue_date);
