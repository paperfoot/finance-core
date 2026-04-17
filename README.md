# finance-core

Shared library for the [Paperfoot](https://github.com/paperfoot) accounting suite.

Every finance CLI — `invoice-cli`, and the forthcoming `receipt-cli`,
`expense-cli`, `bank-cli`, `recon-cli`, `ledger-cli`, `tax-cli` — depends on
this crate. It owns the things every accounting tool agrees on:

| Module     | What it owns                                                                |
|------------|-----------------------------------------------------------------------------|
| `money`    | `MinorUnits` (i64 cents) + `rust_decimal` math helpers + discount math.     |
| `tax`      | `Jurisdiction` enum + `TaxProfile` for SG / UK / US / EU / Custom.          |
| `entity`   | `Issuer` — the canonical "company you transact as" primitive.               |
| `paths`    | Canonical suite dirs via `ProjectDirs("com", "paperfoot", "accounting")`.   |
| `settings` | TOML-backed `Settings` shared across tools (default_issuer, self_update…).  |
| `db`       | Shared SQLite connection + `refinery` migrations for the whole suite.       |
| `error`    | `CoreError` — consumed by each CLI via `#[from]` on its own error enum.     |

## What a consumer looks like

```rust
use finance_core::{db, paths::Paths, settings::Settings};

let paths    = Paths::resolve()?;
let settings = Settings::load(&paths)?;
let conn     = db::open(&paths)?;  // migrations run automatically
// … query `conn` for entities, invoices, receipts, whatever.
```

One SQLite file lives at `Paths::db_file()` (e.g. on macOS
`~/Library/Application Support/com.paperfoot.accounting/accounting.db`) and
every tool in the suite reads/writes the same file — so `invoice new` sees the
same default issuer as `receipt add`, and the ledger is consistent across
tools.

## Versioning discipline

`finance-core` owns the schema. To keep bins that pin different versions from
corrupting each other:

- **Minor bumps** (`0.1.x → 0.2.0`) may add columns, tables, or indexes — never
  remove or rename. Older bins keep working against the newer schema.
- **Major bumps** (`0.x → 1.0`) may change the schema in breaking ways. Every
  bin must be rebuilt before using the new DB.
- Every consumer CLI's `doctor` command should refuse to run if the DB schema
  version is higher than what its pinned `finance-core` version understands.

## License

MIT. See [LICENSE](./LICENSE).
