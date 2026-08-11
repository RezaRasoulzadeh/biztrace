// src/database/mod.rs

mod catalog;
mod inventory;

pub use catalog::{CatalogDraft, CatalogImportResult, CatalogRecord};
pub use inventory::{
    InventoryImportResult, InventoryImportRow, InventoryMovementDraft, MovementRecord, StockRecord,
    WarehouseRecord,
};

use std::env;
use std::error::Error;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use directories::ProjectDirs;
use rusqlite::Connection;

const MIGRATIONS: &[(i64, &str)] = &[
    (1, include_str!("../../migrations/0001_initial.sql")),
    (
        2,
        include_str!("../../migrations/0002_optional_inventory_actor.sql"),
    ),
    (
        3,
        include_str!("../../migrations/0003_inventory_cost_layers.sql"),
    ),
    (
        4,
        include_str!("../../migrations/0004_inventory_layer_revisions.sql"),
    ),
    (5, include_str!("../../migrations/0005_product_skus.sql")),
    (
        6,
        include_str!("../../migrations/0006_inventory_revision_warehouses.sql"),
    ),
];

#[derive(Debug)]
pub enum DatabaseError {
    DataDirectoryUnavailable,
    Io(std::io::Error),
    Sql(rusqlite::Error),
    Validation(String),
}

impl fmt::Display for DatabaseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DataDirectoryUnavailable => {
                formatter.write_str("application data directory is unavailable")
            }
            Self::Io(error) => write!(formatter, "database filesystem error: {error}"),
            Self::Sql(error) => write!(formatter, "database error: {error}"),
            Self::Validation(error) => formatter.write_str(error),
        }
    }
}

impl Error for DatabaseError {}

impl From<std::io::Error> for DatabaseError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<rusqlite::Error> for DatabaseError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Sql(error)
    }
}

pub struct Database {
    connection: Connection,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OverviewCounts {
    pub invoices: i32,
    pub customers: i32,
    pub catalog_items: i32,
    pub warehouses: i32,
    pub fund_accounts: i32,
    pub fund_transactions: i32,
    pub users: i32,
}

impl Database {
    pub fn open_default() -> Result<Self, DatabaseError> {
        let path = Self::default_path()?;
        Self::open(path)
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self, DatabaseError> {
        if let Some(parent) = path.as_ref().parent() {
            fs::create_dir_all(parent)?;
        }
        let connection = Connection::open(path)?;
        Self::initialize(connection)
    }

    pub fn open_in_memory() -> Result<Self, DatabaseError> {
        Self::initialize(Connection::open_in_memory()?)
    }

    pub fn default_path() -> Result<PathBuf, DatabaseError> {
        if let Some(path) = env::var_os("NEXORA_DATABASE_PATH") {
            return Ok(PathBuf::from(path));
        }
        let directories = ProjectDirs::from("com", "Nexora", "Nexora")
            .ok_or(DatabaseError::DataDirectoryUnavailable)?;
        Ok(directories.data_local_dir().join("nexora.sqlite3"))
    }

    pub fn schema_version(&self) -> Result<i64, DatabaseError> {
        self.connection
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
                [],
                |row| row.get(0),
            )
            .map_err(DatabaseError::from)
    }

    pub fn overview_counts(&self) -> Result<OverviewCounts, DatabaseError> {
        Ok(OverviewCounts {
            invoices: self.count_rows("invoices")?,
            customers: self.count_rows("customers")?,
            catalog_items: self.count_rows("catalog_items")?,
            warehouses: self.count_rows("warehouses")?,
            fund_accounts: self.count_rows("fund_accounts")?,
            fund_transactions: self.count_rows("fund_transactions")?,
            users: self.count_rows("users")?,
        })
    }

    fn count_rows(&self, table: &str) -> Result<i32, DatabaseError> {
        let sql = format!("SELECT COUNT(*) FROM {table}");
        self.connection
            .query_row(&sql, [], |row| row.get(0))
            .map_err(DatabaseError::from)
    }

    fn initialize(mut connection: Connection) -> Result<Self, DatabaseError> {
        connection.pragma_update(None, "foreign_keys", "ON")?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.pragma_update(None, "synchronous", "NORMAL")?;
        connection.busy_timeout(Duration::from_secs(5))?;
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );",
        )?;
        let current = connection.query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get::<_, i64>(0),
        )?;
        for &(version, sql) in MIGRATIONS.iter().filter(|(version, _)| *version > current) {
            let transaction = connection.transaction()?;
            transaction.execute_batch(sql)?;
            transaction.execute(
                "INSERT INTO schema_migrations (version) VALUES (?1)",
                [version],
            )?;
            transaction.commit()?;
        }
        Ok(Self { connection })
    }
}
