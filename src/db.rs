// ═══════════════════════════════════════════════════════════════════════════
// DB — shared SQLite connection + refinery migrations.
//
// One SQLite file (`Paths::db_file()`) is used by every finance-* CLI.
// finance-core owns the schema and the migration runner so tables never
// diverge across tools.
// ═══════════════════════════════════════════════════════════════════════════

use std::path::Path;

use rusqlite::Connection;

use crate::error::Result;
use crate::paths::Paths;

mod embedded {
    use refinery::embed_migrations;
    embed_migrations!("./migrations");
}

/// Open the shared accounting DB (creating dirs + file if missing) and run
/// pending migrations. Use this in every CLI that wants to read/write the
/// suite's data.
pub fn open(paths: &Paths) -> Result<Connection> {
    open_at(&paths.db_file())
}

pub fn open_at(path: &Path) -> Result<Connection> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut conn = Connection::open(path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    embedded::migrations::runner().run(&mut conn)?;
    Ok(conn)
}

/// Current applied migration version — useful for `doctor` / health checks.
pub fn schema_version(conn: &Connection) -> Result<Option<u32>> {
    let v: Option<u32> = conn
        .query_row(
            "SELECT MAX(version) FROM refinery_schema_history",
            [],
            |row| row.get(0),
        )
        .ok();
    Ok(v)
}
