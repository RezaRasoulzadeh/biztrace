// src/database/catalog.rs

use rusqlite::{OptionalExtension, params};

use super::{Database, DatabaseError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogRecord {
    pub id: i64,
    pub kind: String,
    pub name: String,
    pub sku: String,
    pub unit: String,
    pub sale_price_minor: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogDraft {
    pub id: Option<i64>,
    pub kind: String,
    pub name: String,
    pub sku: Option<String>,
    pub unit: String,
    pub sale_price_minor: i64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CatalogImportResult {
    pub inserted: usize,
    pub duplicates: usize,
}

impl Database {
    pub fn catalog_items(&self, search: &str) -> Result<Vec<CatalogRecord>, DatabaseError> {
        let pattern = format!("%{}%", search.trim());
        let mut statement = self.connection.prepare(
            "SELECT id, kind, name, COALESCE(sku, ''), unit, sale_price_minor
             FROM catalog_items
             WHERE active = 1 AND (name LIKE ?1 OR COALESCE(sku, '') LIKE ?1)
             ORDER BY updated_at DESC, id DESC",
        )?;
        let records = statement
            .query_map([pattern], |row| {
                Ok(CatalogRecord {
                    id: row.get(0)?,
                    kind: row.get(1)?,
                    name: row.get(2)?,
                    sku: row.get(3)?,
                    unit: row.get(4)?,
                    sale_price_minor: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(records)
    }

    pub fn save_catalog_item(&self, draft: &CatalogDraft) -> Result<i64, DatabaseError> {
        if let Some(id) = draft.id {
            let sku = resolved_sku(&self.connection, &draft.kind, draft.sku.as_deref(), id)?;
            self.connection.execute(
                "UPDATE catalog_items
                 SET kind = ?1, name = ?2, sku = ?3, unit = ?4, sale_price_minor = ?5,
                     track_inventory = CASE WHEN ?1 = 'product' THEN 1 ELSE 0 END,
                     updated_at = CURRENT_TIMESTAMP
                 WHERE id = ?6 AND active = 1",
                params![
                    draft.kind,
                    draft.name,
                    sku,
                    draft.unit,
                    draft.sale_price_minor,
                    id
                ],
            )?;
            return Ok(id);
        }
        let next_id = self.connection.query_row(
            "SELECT COALESCE((SELECT seq FROM sqlite_sequence WHERE name = 'catalog_items'), 0) + 1",
            [],
            |row| row.get::<_, i64>(0),
        )?;
        let sku = resolved_sku(&self.connection, &draft.kind, draft.sku.as_deref(), next_id)?;
        self.connection.execute(
            "INSERT INTO catalog_items
             (kind, name, sku, unit, sale_price_minor, currency, track_inventory)
             VALUES (?1, ?2, ?3, ?4, ?5, 'IRR', CASE WHEN ?1 = 'product' THEN 1 ELSE 0 END)",
            params![
                draft.kind,
                draft.name,
                sku,
                draft.unit,
                draft.sale_price_minor
            ],
        )?;
        Ok(self.connection.last_insert_rowid())
    }

    pub fn catalog_item(&self, id: i64) -> Result<Option<CatalogRecord>, DatabaseError> {
        self.connection
            .query_row(
                "SELECT id, kind, name, COALESCE(sku, ''), unit, sale_price_minor
                 FROM catalog_items WHERE id = ?1 AND active = 1",
                [id],
                |row| {
                    Ok(CatalogRecord {
                        id: row.get(0)?,
                        kind: row.get(1)?,
                        name: row.get(2)?,
                        sku: row.get(3)?,
                        unit: row.get(4)?,
                        sale_price_minor: row.get(5)?,
                    })
                },
            )
            .optional()
            .map_err(DatabaseError::from)
    }

    pub fn remove_catalog_item(&self, id: i64) -> Result<(), DatabaseError> {
        self.connection
            .execute("DELETE FROM catalog_items WHERE id = ?1", [id])?;
        Ok(())
    }

    pub fn import_catalog_items(
        &self,
        drafts: &[CatalogDraft],
    ) -> Result<CatalogImportResult, DatabaseError> {
        let transaction = self.connection.unchecked_transaction()?;
        let mut result = CatalogImportResult::default();
        {
            let mut duplicate_check = transaction.prepare(
                "SELECT EXISTS(
                    SELECT 1 FROM catalog_items
                    WHERE active = 1 AND (
                        (?1 IS NOT NULL AND sku = ?1 COLLATE NOCASE)
                        OR (?1 IS NULL AND kind = ?2 AND name = ?3 AND unit = ?4)
                    )
                )",
            )?;
            let mut statement = transaction.prepare(
                "INSERT INTO catalog_items
                 (kind, name, sku, unit, sale_price_minor, currency, track_inventory)
                 VALUES (?1, ?2, ?3, ?4, ?5, 'IRR', CASE WHEN ?1 = 'product' THEN 1 ELSE 0 END)
                 ON CONFLICT(sku) DO NOTHING",
            )?;
            for draft in drafts {
                let duplicate = duplicate_check.query_row(
                    params![draft.sku, draft.kind, draft.name, draft.unit],
                    |row| row.get::<_, bool>(0),
                )?;
                if duplicate {
                    result.duplicates += 1;
                    continue;
                }
                let next_id = transaction.query_row(
                    "SELECT COALESCE((SELECT seq FROM sqlite_sequence WHERE name = 'catalog_items'), 0) + 1",
                    [],
                    |row| row.get::<_, i64>(0),
                )?;
                let sku = resolved_sku(&transaction, &draft.kind, draft.sku.as_deref(), next_id)?;
                let changed = statement.execute(params![
                    draft.kind,
                    draft.name,
                    sku,
                    draft.unit,
                    draft.sale_price_minor
                ])?;
                if changed == 0 {
                    result.duplicates += 1;
                } else {
                    result.inserted += 1;
                }
            }
        }
        transaction.commit()?;
        Ok(result)
    }
}

fn resolved_sku(
    connection: &rusqlite::Connection,
    kind: &str,
    supplied: Option<&str>,
    id: i64,
) -> Result<Option<String>, DatabaseError> {
    if let Some(sku) = supplied.map(str::trim).filter(|sku| !sku.is_empty()) {
        return Ok(Some(sku.to_owned()));
    }
    if kind != "product" {
        return Ok(None);
    }
    let base = format!("NXR-P-{id:08}");
    for suffix in 0..1_000 {
        let candidate = if suffix == 0 {
            base.clone()
        } else {
            format!("{base}-{suffix}")
        };
        let exists = connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM catalog_items WHERE sku = ?1 COLLATE NOCASE AND id <> ?2)",
            params![candidate, id],
            |row| row.get::<_, bool>(0),
        )?;
        if !exists {
            return Ok(Some(candidate));
        }
    }
    Err(DatabaseError::Validation(
        "unable to generate a unique product sku".into(),
    ))
}
