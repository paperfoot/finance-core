# Paperfoot Accounting Suite — Proposal for the Next 4 CLIs

_Drafted: 2026-04-18. Foundation already shipped: `finance-core` 0.1.0 + `invoice-cli` 0.3.0._

This document captures the concrete plan for extending the suite beyond
invoice generation into a minimum-useful accounting stack for a solo
founder / small company operator. It's deliberately opinionated about
scope — we keep the suite to 5 CLIs total (including invoice), not 7+,
because the additional tools don't earn their maintenance cost for a
single-operator workflow.

## Context

Boris runs Paperfoot AI (SG) Pte. Ltd. (and is founder of 199
Biotechnologies). Primary accounting needs:

- **Money in**: invoice clients → track payment → GST on sales
- **Money out**: track expenses → keep receipts → GST on purchases
- **Cash position**: reconcile against actual bank balances
- **Compliance**: quarterly SG GST F5 filings (+ UK VAT if UK entity exists)
- **Reporting**: annual P&L + balance sheet handoff to accountant

Everything else (payroll, AR aging beyond what invoice-cli already does,
double-entry ledger, multi-user access, inventory) is out of scope.

## Suite at a glance

| # | Crate         | Role                                          | Status          |
|---|---------------|-----------------------------------------------|-----------------|
| — | `finance-core`| Shared types, schema, DB, settings            | ✅ 0.1.0 shipped |
| 1 | `invoice-cli` | Money in — bill clients, track A/R           | ✅ 0.3.0 shipped |
| 2 | `expense-cli` | Money out + receipt OCR                       | Proposed         |
| 3 | `bank-cli`    | Bank statement import + reconciliation        | Proposed         |
| 4 | `tax-cli`     | GST F5 / VAT / OSS filing packs               | Proposed         |
| 5 | `report-cli`  | P&L, balance sheet, cash flow                 | Proposed         |

### Why not also `receipt-cli`, `recon-cli`, `ledger-cli`?

- **receipt-cli folded into expense-cli**. A receipt is just an attachment
  on an expense. Two CLIs is over-engineering for a solo operator; one
  tool with `expense new --photo path.jpg` handles both photo OCR and
  expense ledger in one step.
- **recon-cli folded into bank-cli**. Reconciliation is inherently a bank
  operation (matching bank lines to invoices/expenses). No value in a
  separate binary.
- **ledger-cli deliberately skipped**. Double-entry primitives are what
  accountants love; solo founders don't touch them. The annual CSV export
  of transactions from invoice + expense covers the accountant handoff
  need. Revisit only if compliance complexity grows (multi-entity
  consolidation, audit requirements).

## Build order + rationale

Strict sequence — each builds on the previous:

1. **expense-cli** — daily-use tool, unlocks everything downstream.
2. **tax-cli** — first real deadline pressure (SG GST F5 is quarterly).
3. **bank-cli** — polish layer; you can file taxes without it, but not
   confidently.
4. **report-cli** — year-end need; accountant deadline driven.

---

## CLI 1 of 4 · `expense-cli`

### Purpose

Track every dollar leaving the business. Categorize, tag with tax rate,
attach receipts (photo → OCR → auto-populate). Query by period, vendor,
category. Export for GST filings and accountant handoff.

### Why first

1. **Used daily**. Unlike invoices (a few times a month), expenses
   happen every coffee, every cloud bill, every flight. Building the
   habit now captures value starting day one.
2. **Unblocks tax-cli**. SG GST F5 line 5 = input tax (GST paid on
   purchases). Without expense-cli this field is guesswork.
3. **Unblocks bank-cli**. Reconciliation matches bank lines against
   invoices (money in) *and* expenses (money out). Half the pipeline
   is missing without expenses.
4. **Clear "afternoon scope"**. Schema is one small table plus
   categories. OCR pipeline already exists in the
   `local-vision-lab-extraction` skill — same MLX/Qwen3-VL pattern,
   different ontology.

### Command surface

```
expense new --amount <N> --vendor <NAME> --category <CAT> [--photo PATH] [--date today|YYYY-MM-DD] [--tax-rate <R>] [--notes ...] [--as <issuer>]
expense list [--from ... --to ... --category X --vendor Y --as Z --unmatched]
expense show <id>
expense edit <id> [--amount ... --vendor ... --category ... --tax-rate ...]
expense delete <id>
expense from-receipt <photo> [--dry-run]   # OCR → structured → create
expense categories list|add|remove         # taxonomy CRUD
expense vendors list|add|edit|delete       # reuses finance-core entities (kind=merchant)
expense export --from ... --to ... --format csv|json [--out PATH]
expense agent-info | doctor | update | skill install
```

Invoice-cli CRUD style preserved — JSON envelope, TTY auto-detect,
exit codes 0/1/2/3, agent-info manifest, embedded SKILL.md.

### Schema additions (finance-core v0.2.0, additive migration V5)

```sql
-- Vendors: extend entities rather than duplicate. For the MVP we add
-- a `kind` column ('issuer' | 'client' | 'merchant') to the existing
-- entities table so one list covers all three roles across the suite.
ALTER TABLE issuers ADD COLUMN kind TEXT NOT NULL DEFAULT 'issuer';
-- Future rename: `issuers` → `entities`. Deferred because it touches
-- every query in invoice-cli/src/db.rs.

-- Expense categories (flat for MVP; parent_code for future hierarchy).
CREATE TABLE expense_categories (
    code         TEXT PRIMARY KEY,   -- 'meals', 'cloud', 'travel', …
    name         TEXT NOT NULL,      -- display name
    parent_code  TEXT REFERENCES expense_categories(code),
    gst_treatment TEXT DEFAULT 'standard'  -- 'standard'|'exempt'|'zero-rated'|'out-of-scope'
);

-- The main event.
CREATE TABLE expenses (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    issuer_id         INTEGER NOT NULL REFERENCES issuers(id),  -- WHICH of your companies paid
    date              TEXT NOT NULL,
    vendor_id         INTEGER REFERENCES issuers(id),           -- kind='merchant'
    vendor_text       TEXT,                                      -- fallback if no matched vendor
    category_code     TEXT NOT NULL REFERENCES expense_categories(code),
    amount_minor      INTEGER NOT NULL,
    currency          TEXT NOT NULL,
    tax_rate          TEXT,                                      -- '9' for 9% GST
    tax_amount_minor  INTEGER,                                   -- denormalised for fast queries
    reimbursable      INTEGER NOT NULL DEFAULT 0,
    notes             TEXT,
    receipt_sha       TEXT REFERENCES attachments(sha256),
    created_at        TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_expenses_date      ON expenses(date);
CREATE INDEX idx_expenses_category  ON expenses(category_code);
CREATE INDEX idx_expenses_issuer    ON expenses(issuer_id);

-- Attachments table serves expenses today, invoice logos/PDFs later.
-- Content-addressed by SHA-256 so the same receipt photo referenced
-- twice dedupes on disk.
CREATE TABLE attachments (
    sha256         TEXT PRIMARY KEY,
    mime           TEXT NOT NULL,
    size_bytes     INTEGER NOT NULL,
    original_name  TEXT,
    stored_rel     TEXT NOT NULL,      -- relative to assets_dir/attachments/
    created_at     TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

### OCR pipeline

1. `expense from-receipt photo.jpg` loads the image.
2. Hashes the file (SHA-256), copies into
   `<paths.assets_dir()>/attachments/<sha>.<ext>`, inserts the
   `attachments` row.
3. Invokes Qwen3-VL via MLX (reusing the pattern from
   `local-vision-lab-extraction` skill) with a prompt that extracts:
   `{ vendor, date, total, tax_rate, tax_amount, currency, line_items[] }`.
4. Presents the parsed result to the user (human stream) or JSON envelope
   (agent stream). `--dry-run` just prints; without it, opens a
   pre-filled `expense new` that the user can confirm.
5. Stores `receipt_sha` on the new expense row, closing the loop.

Accuracy target: 95%+ on merchant-printed receipts (cafés, ride-share,
hotels). Hand-written receipts will need manual correction — that's
fine; the CLI is fast enough to edit.

### Seed categories (jurisdiction-aware)

A one-time seed populates SG-aware defaults:

| Code            | Name                  | GST treatment  |
|-----------------|-----------------------|----------------|
| `meals`         | Meals & entertainment | standard (partial claim) |
| `travel`        | Travel                | standard       |
| `cloud`         | Cloud + SaaS          | standard       |
| `hardware`      | Equipment & hardware  | standard       |
| `office`        | Office supplies       | standard       |
| `contractors`  | Contractors           | standard / zero-rated (if overseas) |
| `legal`         | Legal & accounting    | standard       |
| `marketing`     | Marketing & ads       | standard       |
| `subscriptions` | Subscriptions         | standard       |
| `bank-fees`     | Bank fees             | exempt         |
| `taxes`         | Taxes paid            | out-of-scope   |

### Dependencies

- `finance-core = "0.2"` (needs the new expenses + attachments + categories tables)
- Local MLX + Qwen3-VL via the existing skill (invoked by shell-out)
- Optional: `imagemagick` or `libheif` for HEIC → JPEG if iPhone photos

### Open questions

1. **Multi-currency** — if Boris pays a USD cloud bill from a SGD
   account, do we store USD amount + SGD conversion? Probably
   both. FX source: ECB reference rates as of the date.
2. **Recurring expenses** (Netflix, cloud subscriptions): template
   mechanism? `expense recurring add --monthly --amount ...` then a
   `expense recurring run` to materialize for a month. Deferred — do
   it after basic CRUD ships.
3. **Personal vs business**: flag on expense or separate issuer? Use
   a separate issuer slug `personal` and filter with `--as personal`
   in queries. Cleaner than a boolean.
4. **Splits** (one receipt, multiple categories) — rare enough to
   skip in MVP. Use two expense rows with same `receipt_sha`.

### Effort estimate

- Session 1: crate scaffold, CRUD, categories seed, JSON + TTY, agent-info. **MVP.**
- Session 2: receipt attach + OCR integration + exports. **Useful.**
- Session 3: Multi-currency + recurring + polish. **Complete.**

---

## CLI 2 of 4 · `tax-cli`

### Purpose

Take invoice-cli revenue + expense-cli purchases and produce filing-ready
GST F5 (SG), VAT Return (UK), or EU OSS submissions. Also: quarterly
estimates, deadline reminders, and year-end prep for income tax.

### Why second (not third)

Deadline-driven. SG GST F5 is quarterly; missing one = penalty. The
moment expense-cli is producing clean data, tax-cli is the *forcing
function* for keeping it clean. Build it before the first GST period
after expense-cli ships.

### Command surface

```
tax gst-f5 --quarter 2026-Q1 --as acme [--out PATH]
    # Generate SG GST F5 filing pack: 7-box CSV + PDF summary
tax vat-return --period 2026-Q1 --as acme-uk [--out PATH]
    # UK MTD VAT return (9 boxes)
tax report --jurisdiction sg --from ... --to ... --as ...
    # Raw aggregate report (input/output tax, net position)
tax estimate --year 2026 [--as ...]
    # Project annual corporate tax liability from YTD numbers
tax deadlines [--as ...]
    # List upcoming filing deadlines across issuers
tax filings list [--status filed|draft|overdue]
    # All past + pending filings
tax mark-filed <filing-id> --ref IRAS-2026-Q1-XXXX
tax agent-info | doctor | update | skill install
```

### Schema additions (finance-core v0.3.0, migration V6)

```sql
CREATE TABLE tax_filings (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    issuer_id       INTEGER NOT NULL REFERENCES issuers(id),
    jurisdiction    TEXT NOT NULL,           -- 'sg', 'uk', 'eu'
    period          TEXT NOT NULL,           -- '2026-Q1', '2026-03', '2026'
    kind            TEXT NOT NULL,           -- 'gst-f5', 'vat-return', 'eu-oss', 'corp-tax'
    status          TEXT NOT NULL DEFAULT 'draft',  -- draft|filed|amended
    filed_at        TEXT,
    external_ref    TEXT,                    -- IRAS submission number, HMRC ref, etc.
    data_snapshot   TEXT NOT NULL,           -- JSON blob of all computed boxes
    created_at      TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (issuer_id, jurisdiction, period, kind)
);
```

### SG GST F5 logic

Seven boxes to populate from existing data:

| Box | Name                        | Source                                           |
|-----|-----------------------------|--------------------------------------------------|
| 1   | Total value of std-rated supplies | `SUM(invoices.total_pretax)` where issued, tax_rate>0 |
| 2   | Total value of zero-rated supplies | `SUM(invoices.total_pretax)` where tax_rate=0 (overseas clients) |
| 3   | Total value of exempt supplies | Usually 0 for most SaaS; separate invoice tag   |
| 4   | Total supplies (1+2+3)       | computed                                          |
| 5   | Taxable purchases             | `SUM(expenses.amount_pretax)` where GST-claimable |
| 6   | Output tax                    | `SUM(invoices.tax_amount)` where issued          |
| 7   | Input tax + refunds claimed   | `SUM(expenses.tax_amount)` where claimable       |
| Net | Box 6 − Box 7                 | payable to (or refundable from) IRAS              |

Output: CSV matching IRAS format + PDF summary with line-item audit
trail + JSON blob stored in `tax_filings.data_snapshot`.

### UK VAT (for later — if a UK entity is ever set up)

Similar shape, 9 boxes, Making Tax Digital (MTD) compliance means
submissions go through HMRC's API — defer this entire jurisdiction
until there's a UK issuer. Stub the command; error with "UK VAT
filing not yet implemented" for now.

### Dependencies

- `finance-core = "0.3"` (adds tax_filings table)
- `invoice-cli = "0.3"` (read invoices)
- `expense-cli = "0.1"` (read expenses)

### Open questions

1. **Cash vs accrual basis** — SG allows cash accounting under $1M turnover.
   Default to accrual (invoice issue date) since that matches what
   invoice-cli already does; add `--basis cash` flag later.
2. **Bad debt relief** — write-off of unpaid invoices as bad debt.
   Rare; expose `invoices mark void` + let tax-cli exclude voided
   invoices from box 1. Good enough.
3. **Partial exemption** — if some revenue is exempt, input tax needs
   apportionment. Defer; flag in the data_snapshot when detected.
4. **Reverse charge** (already in invoice-cli) — how does it affect
   boxes 1 and 6? Needs jurisdiction-specific logic.

### Effort estimate

- Session 1: SG GST F5 CSV generation + `tax report` + `tax deadlines`. **MVP.**
- Session 2: Polish — PDF output, amendment workflow, estimates. **Complete for SG.**
- Session 3+: UK VAT via MTD API, EU OSS. **Future.**

---

## CLI 3 of 4 · `bank-cli`

### Purpose

Import bank + credit-card statements (CSV / OFX / MT940). Reconcile
every line against invoices (money in) and expenses (money out).
Flag the unmatched ones so nothing slips through. Track actual cash
balance per account.

### Why third

Polish over necessity. You *can* file GST without bank reconciliation
if you trust your invoice + expense data. But "trust" is the issue —
bank-cli is the audit layer that catches missing entries before they
cost you at year-end.

### Command surface

```
bank accounts list|add|edit|delete
    # Which bank/credit-card accounts to track
bank import --account <slug> --file statement.csv [--format dbs|revolut|wise|ofx|generic-csv]
bank list [--account ... --unmatched --from ... --to ...]
bank reconcile [--account ...] [--from ... --to ...]
    # Auto-match against invoices + expenses by amount + date
bank assign <txn-id> invoice:<number> | expense:<id>
    # Manual match for ambiguous cases
bank unassign <txn-id>
bank balance [--account ...] [--as-of YYYY-MM-DD]
bank transfer <from-account> <to-account> --amount ... --date ...
    # Internal transfers (don't count as income/expense)
bank agent-info | doctor | update | skill install
```

### Schema additions (finance-core v0.4.0, migration V7)

```sql
CREATE TABLE bank_accounts (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    slug           TEXT NOT NULL UNIQUE,
    name           TEXT NOT NULL,
    issuer_id      INTEGER NOT NULL REFERENCES issuers(id),
    currency       TEXT NOT NULL,
    institution    TEXT,
    number_masked  TEXT,          -- '****1234'
    kind           TEXT NOT NULL,  -- 'checking'|'savings'|'credit_card'
    opening_balance_minor INTEGER NOT NULL DEFAULT 0,
    opening_as_of  TEXT,           -- ISO date for the opening balance
    active         INTEGER NOT NULL DEFAULT 1,
    created_at     TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE bank_transactions (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id     INTEGER NOT NULL REFERENCES bank_accounts(id),
    date           TEXT NOT NULL,
    description    TEXT NOT NULL,
    amount_minor   INTEGER NOT NULL,   -- negative = debit, positive = credit
    balance_minor  INTEGER,
    external_id    TEXT,               -- bank's own txn ref for dedup
    matched_kind   TEXT,               -- 'invoice' | 'expense' | 'transfer' | null
    matched_id     INTEGER,            -- FK into invoices / expenses / bank_transactions
    notes          TEXT,
    imported_at    TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (account_id, external_id)
);

CREATE INDEX idx_bank_txn_date     ON bank_transactions(date);
CREATE INDEX idx_bank_txn_matched  ON bank_transactions(matched_kind, matched_id);
```

### Reconciliation algorithm (greedy, explicit)

For each unmatched transaction, in date order:

1. **Exact match candidate on amount + date ± 2 days**:
   - If `amount > 0` (credit) → look for invoice where `status='issued'`
     and `total_minor == amount`. If exactly one match, auto-assign +
     `invoice mark paid`.
   - If `amount < 0` (debit) → look for expense where
     `amount_minor == -amount` and `date` within window. If exactly
     one match, auto-assign.
2. **Ambiguous (multiple matches)**: leave unmatched, surface in
   `bank list --unmatched` with candidates in a `suggestions` field.
3. **No match**: leave unmatched. User either creates a new expense
   (`expense new` then `bank assign`) or marks it as a transfer/fee.

Never auto-assign an ambiguous match. The UX cost of a wrong auto-match
(silent data corruption) is worse than the UX cost of one manual
confirm.

### Bank format zoo

Start with **DBS SG** (the most common for Singapore founders) CSV
format. Add others on-demand:

| Bank / format      | Priority | Notes                                     |
|--------------------|----------|-------------------------------------------|
| DBS SG CSV         | P0       | Boris's primary                            |
| Generic CSV        | P1       | date / desc / amount — user maps columns  |
| Wise / Revolut CSV | P2       | Multi-currency                             |
| OFX (OpenFinX)     | P2       | Some US/UK banks                           |
| MT940              | P3       | EU wire format                             |
| CAMT.053           | P4       | Modern EU replacement for MT940            |
| Plaid API          | out of scope | US-only, adds auth complexity         |

### Dependencies

- `finance-core = "0.4"` (adds bank tables)
- `invoice-cli` + `expense-cli` for matching (at runtime, not build)

### Open questions

1. **Multi-currency accounts**: if USD account is used to pay a SGD
   expense, how does matching work? Probably: match on converted
   amount (using the expense's stored FX rate) with tolerance.
2. **Credit card ↔ bank payment**: paying the CC is itself a bank
   transaction. Handled by the `bank transfer` command linking both
   sides so they cancel out in reports.
3. **Initial balance** — user must specify opening balance per
   account to get meaningful running balances.

### Effort estimate

- Session 1: accounts CRUD + DBS CSV import + basic `bank list`. **MVP.**
- Session 2: Reconciliation algorithm + manual assign. **Useful.**
- Session 3: Generic CSV mapper + multi-currency + transfer handling.

---

## CLI 4 of 4 · `report-cli`

### Purpose

Period-based financial reports ready for accountant handoff, board
slides, or just your own understanding. P&L, balance sheet, cash flow.
Annual pack for year-end close.

### Why last

All the data is already in the shared DB by the time the other three
CLIs are live. report-cli is mostly SQL + formatting — no new schema,
no new integrations. It's the thing you run once a quarter (for boards)
and once a year (for the accountant), not daily.

### Command surface

```
report pnl --from ... --to ... --as <issuer> [--format md|pdf|csv] [--out PATH]
report balance-sheet --as-of YYYY-MM-DD --as <issuer> [--format ...]
report cash-flow --from ... --to ... --as <issuer> [--format ...]
report annual --year 2026 --as <issuer>
    # Bundle of P&L + BS + CF for the year
report compare --period 2026-Q1 --vs 2025-Q1 --as <issuer>
    # Period-over-period variance
report dashboard [--as <issuer>]
    # Current cash, YTD revenue, YTD expenses, A/R outstanding
report agent-info | doctor | update | skill install
```

### Report compositions

**P&L (accrual basis, default)**:
```
Revenue                                                SGD 000
  Consulting                                         120,000
  Product sales                                       45,000
                                                     -------
                                                     165,000
Expenses
  Cloud + SaaS                                        24,000
  Contractors                                         30,000
  Travel                                              18,000
  …
                                                     -------
                                                      92,000
                                                     =======
Net profit before tax                                 73,000
```

**Balance sheet** (as of YYYY-MM-DD):
```
Assets
  Cash at bank (per bank-cli)                         45,000
  A/R (unpaid issued invoices)                        20,000
                                                     -------
                                                      65,000
Liabilities
  A/P (future: recurring/unpaid bills)                 5,000
  GST payable (this quarter)                           3,000
Equity                                                57,000
```

**Cash flow** (from bank transactions):
```
Operating: invoices paid − expenses paid − GST paid
Investing: equipment purchases (category=hardware), etc.
Financing: founder loans, dividends, tax instalments
```

### Schema additions

None. Pure read layer.

### Dependencies

- `finance-core >= 0.4`
- All four upstream CLIs for full richness:
  - `invoice-cli` (revenue side)
  - `expense-cli` (costs side)
  - `bank-cli` (cash position for BS)
  - `tax-cli` (GST liability for BS)

### Open questions

1. **Accrual vs cash basis**: offer both via `--basis accrual|cash`
   flag. Accountants want accrual; tax filings in SG often use cash.
2. **Multi-currency consolidation**: if issuer has USD + SGD invoices,
   BS in SGD at period-end FX rate. Document the rate used.
3. **Inter-company transactions**: if Paperfoot SG bills 199 Bio UK,
   those should net to zero at the group level. Flag for future.
4. **Comparatives**: default to showing prior period alongside current?
   Probably yes for BS and P&L; adds scannability.

### Effort estimate

- Session 1: P&L + balance sheet in Markdown. **MVP.**
- Session 2: Cash flow + Typst PDF rendering + annual bundle.
- Session 3: Comparisons + dashboard + multi-currency consolidation.

---

## Cross-suite considerations

### finance-core version cadence

Each CLI above triggers a minor bump in finance-core (additive schema
migration). Rough projection:

| finance-core | Adds                                            | Trigger CLI     |
|--------------|-------------------------------------------------|-----------------|
| 0.1          | money, tax, entity, settings, paths, db, error | invoice-cli v0.3|
| 0.2          | expenses, attachments, expense_categories      | expense-cli     |
| 0.3          | tax_filings                                    | tax-cli         |
| 0.4          | bank_accounts, bank_transactions               | bank-cli        |
| 0.5          | (nothing — report-cli is read-only)            | report-cli      |

Each bump opens Dependabot PRs in every consumer CLI within a week.

### Multi-currency strategy

Pick one **reporting currency** (SGD for Paperfoot) in settings:
```toml
reporting_currency = "SGD"
```

Every transaction stores its native amount + currency. When a report
needs a consolidated view, it converts using a stored FX rate (capture
at transaction time) or the period-end rate. Source: ECB reference
rates, fetched daily by a small shared helper in finance-core.

### Accountant handoff format

Universal: `csv` export from every CLI with a consistent row shape:
`date | account | description | debit | credit | reference`. This is
what accounting software imports. report-cli's `--format csv` feeds
the same.

### Skills install across CLIs

Each CLI should install its own `~/.claude/skills/<name>/SKILL.md`
via `<cli> skill install` (the pattern invoice-cli already uses). A
unified skill at `~/.claude/skills/paperfoot-accounting/` that loads
them all could come later as a convenience.

## Explicitly out of scope

Not building any of these. Revisit only if the business materially changes:

- **Payroll** — use Gusto / Employment Hero; not building one
- **Double-entry ledger CLI** — use the CSV export to feed accountants
- **Inventory management** — not applicable to service/software businesses
- **Multi-user / permissions** — single operator; SQLite file is enough
- **Real-time sync / SaaS hosted version** — CLI-first, file-based is fine
- **Direct bank API integration (Plaid, TrueLayer)** — CSVs are fine, APIs add auth complexity
- **Mobile app** — CLI runs on the phone via Termux if needed; don't build Swift/Kotlin
- **Web UI** — the PDFs generated by invoice/report are the "UI" for non-technical recipients

## Risk register

| Risk                                            | Likelihood | Impact | Mitigation                                      |
|-------------------------------------------------|------------|--------|-------------------------------------------------|
| finance-core schema migration breaks old bins   | low        | high   | SemVer discipline: additive-only in minor bumps |
| OCR accuracy low on handwritten / photos        | medium     | low    | Always allow manual override; don't auto-commit |
| Tax rule drift (SG GST rate changes)            | low        | high   | Seed tax rates in DB; `tax --rate` override    |
| DBS CSV format changes                          | medium     | medium | Column mapper + format versioning              |
| Single SQLite file corruption                   | low        | high   | WAL mode + periodic `VACUUM` + backup command  |

## What to do when you come back to this doc

1. Re-read this section and the suite-at-a-glance table.
2. Decide: is the "next 4" scope still right, or has life changed?
3. If expense-cli still the right next move, start it:
   - `cargo new --lib expense-cli` under `paperfoot/expense-cli`
   - Add `finance-core = "0.2"` dep (bump finance-core first with V5 migration)
   - Scaffold CRUD with invoice-cli as the template
4. Revisit this proposal as a living doc — update the schema sections
   as they ship, strike through the "proposed" markers.

---

## Appendix A — invoice-cli follow-up work (added 2026-04-18)

Done in v0.5.0 / finance-core 0.3.0:

- ✅ Per-issuer default PDF output directory (`issuer edit --output-dir ...`)
- ✅ PDF archive on every render (`<data_dir>/rendered/<year>/<number>.pdf`)
- ✅ Per-issuer default notes with auto-inheritance on new invoices
- ✅ Currency ↔ symbol auto-linking (`--currency GBP` → `£` automatically)
- ✅ Invoice-level currency override with symbol re-derivation when it
  differs from the issuer's default

**Deferred to a future session** — big enough to deserve their own commit
+ test + release round. Roughly in the order I'd tackle them if asked:

1. **Multi-bank-account per issuer.** Today's `bank_details` is one text
   block per issuer. A company with SGD + GBP + USD accounts needs each
   invoice to pick the right account by currency. Fix: new
   `bank_accounts` table (issuer_id, slug, currency, bank_details,
   is_default) + `issuer bank add/list/set-default` subcommand +
   automatic selection at invoice creation time based on the invoice's
   currency, with `--bank-account <slug>` manual override.

2. **Logo portability via the `attachments` table.** Current `logo_path`
   is a filesystem string that breaks on machine moves and CI. Already
   planned as part of expense-cli scope — but can arrive earlier as a
   pure invoice-cli refactor: `issuer set-logo <path>` hashes the image,
   stores at `<data_dir>/attachments/<sha>.<ext>`, sets `logo_sha` on
   the issuer. Typst template reads the sha-named file.

3. **`tax_registrations` as a proper table.** Replaces today's boolean
   `tax_registered` + single `tax_id` with `(issuer_id, jurisdiction,
   number, effective_from)`. Lets Paperfoot carry both a SG-GST number
   (if registered in the future) AND a UK-VAT number in one row each.
   Invoice picks the right one based on the invoice's tax context.

4. **Structured client address.** `country_code`, `city`, `postcode`,
   `street_lines[]` — enables "all UK clients" queries and template
   localisation ("Attn:" → German "z.H.").

5. **Template localisation.** Per-invoice `--language en|de|fr|ja`
   overlays a dictionary (Rechnung / Total / Date / …). Typst already
   supports per-locale dicts — plumb them in + add a `lang` field to
   the invoice.

6. **Audit log.** `audit_events` table logging every CRUD against
   invoices + issuers + clients. `invoice audit <number>` replays the
   history. Compliance gold, cheap to implement.

7. **Deposit / milestone / pro-forma workflow.** `invoices new
   --kind pro-forma` produces a non-sequential informational doc;
   `invoices new --deposit 25%` creates a partial invoice linked back
   to the master. Milestone splits handled as multiple linked invoices.

8. **Recurring / subscription billing.** `invoices recurring add
   --template 2026-0003 --monthly --next-run 2026-05-01`. Matching
   daemon command (`invoice daemon run-recurring`) materialises the
   next batch into real invoices. Simple cron.

9. **Default notes library.** Named note templates per issuer:
   `notes-template add reverse-charge-uk "…"`, then `invoices new
   --notes-template reverse-charge-uk`. Avoids retyping boilerplate
   while still allowing per-invoice overrides.

10. **Number-series flexibility.** Strictly-sequential-across-years
    format for certain jurisdictions; `issuer edit --number-format
    "INV-{seq:06}"` already supports custom formats, but seq-reset
    semantics need a per-issuer toggle.

## Appendix B — general edge-case principles

Surfaced during the today's design conversation — worth remembering:

- **Defaults that work for the first user case rarely scale.** The
  IBAN-centric bank fields worked for EU users, broke for SG. The
  `currency` field on issuer "works" for single-currency businesses,
  breaks for cross-border. Every field defaulted from a single country
  is a future bug.
- **Boolean flags that conflate multiple states.** `tax_registered:
  bool` couldn't capture "not SG-GST-registered AND not UK-VAT-
  registered AND might register for UK VAT later". A table with rows
  per registration scales; a boolean doesn't.
- **Paths are fragile.** Filesystem paths (logos, PDFs) break when
  the user moves machines or runs from CI. Content-addressed blobs or
  DB-stored content is portable.
- **Outputs without archives lose audit trail.** `/tmp/*.pdf` is dev,
  not production. Every artefact with legal or financial weight needs
  an immutable copy alongside the mutable one the user interacts with.

---

_This doc lives at `finance-core/docs/proposal-next-clis.md`. Update in
place when scope changes._
