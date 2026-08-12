// src/database/inventory.rs

use rusqlite::{OptionalExtension, params};

use super::{Database, DatabaseError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WarehouseRecord {
    pub id: i64,
    pub name: String,
    pub address: String,
    pub has_stock: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StockRecord {
    pub cost_layer_id: i64,
    pub warehouse_id: i64,
    pub warehouse_name: String,
    pub item_id: i64,
    pub item_name: String,
    pub sku: String,
    pub unit: String,
    pub quantity_milliunits: i64,
    pub acquired_quantity_milliunits: i64,
    pub inventory_value_minor: i64,
    pub unit_cost_minor: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InventoryMovementDraft {
    pub warehouse_id: i64,
    pub item_id: i64,
    pub cost_layer_id: Option<i64>,
    pub quantity_milliunits: i64,
    pub increases_stock: bool,
    pub unit_cost_minor: Option<i64>,
    pub reference: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MovementRecord {
    pub item_name: String,
    pub warehouse_name: String,
    pub quantity_milliunits: i64,
    pub increases_stock: bool,
    pub reference: String,
    pub occurred_on: String,
    pub unit_cost_minor: Option<i64>,
    pub total_cost_minor: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InventoryImportRow {
    pub warehouse: String,
    pub sku: String,
    pub quantity_milliunits: i64,
    pub unit_cost_minor: i64,
    pub reference: Option<String>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct InventoryImportResult {
    pub inserted: usize,
    pub skipped: usize,
}

impl Database {
    pub fn import_inventory_rows(
        &self,
        rows: &[InventoryImportRow],
    ) -> Result<InventoryImportResult, DatabaseError> {
        let mut result = InventoryImportResult::default();
        for row in rows {
            let warehouse_id = self
                .connection
                .query_row(
                    "SELECT id FROM warehouses WHERE active = 1 AND name = ?1 COLLATE NOCASE",
                    [&row.warehouse],
                    |record| record.get::<_, i64>(0),
                )
                .optional()?;
            let item_id = self
                .connection
                .query_row(
                    "SELECT id FROM catalog_items
                     WHERE active = 1 AND kind = 'product' AND sku = ?1 COLLATE NOCASE",
                    [&row.sku],
                    |record| record.get::<_, i64>(0),
                )
                .optional()?;
            let (Some(warehouse_id), Some(item_id)) = (warehouse_id, item_id) else {
                result.skipped += 1;
                continue;
            };
            let draft = InventoryMovementDraft {
                warehouse_id,
                item_id,
                cost_layer_id: None,
                quantity_milliunits: row.quantity_milliunits,
                increases_stock: true,
                unit_cost_minor: Some(row.unit_cost_minor),
                reference: row.reference.clone(),
            };
            if self.record_inventory_movement(&draft).is_ok() {
                result.inserted += 1;
            } else {
                result.skipped += 1;
            }
        }
        Ok(result)
    }

    pub fn warehouses(&self) -> Result<Vec<WarehouseRecord>, DatabaseError> {
        let mut statement = self.connection.prepare(
            "SELECT w.id, w.name, COALESCE(w.address, ''),
                    EXISTS(SELECT 1 FROM stock_levels s
                           WHERE s.warehouse_id = w.id AND s.quantity_milliunits > 0)
             FROM warehouses w
             WHERE w.active = 1 ORDER BY w.name COLLATE NOCASE",
        )?;
        Ok(statement
            .query_map([], |row| {
                Ok(WarehouseRecord {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    address: row.get(2)?,
                    has_stock: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?)
    }

    pub fn create_warehouse(
        &self,
        name: &str,
        address: Option<&str>,
    ) -> Result<i64, DatabaseError> {
        self.connection.execute(
            "INSERT INTO warehouses (name, address) VALUES (?1, ?2)",
            params![name, address],
        )?;
        Ok(self.connection.last_insert_rowid())
    }

    pub fn save_warehouse(
        &self,
        id: Option<i64>,
        name: &str,
        address: Option<&str>,
    ) -> Result<i64, DatabaseError> {
        if let Some(id) = id {
            self.connection.execute(
                "UPDATE warehouses SET name = ?1, address = ?2 WHERE id = ?3",
                params![name, address, id],
            )?;
            Ok(id)
        } else {
            self.create_warehouse(name, address)
        }
    }

    pub fn remove_warehouse(&self, id: i64) -> Result<(), DatabaseError> {
        let has_stock = self.connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM stock_levels
             WHERE warehouse_id = ?1 AND quantity_milliunits > 0)",
            [id],
            |row| row.get::<_, bool>(0),
        )?;
        if has_stock {
            return Err(DatabaseError::Validation("warehouse contains stock".into()));
        }
        self.connection
            .execute("UPDATE warehouses SET active = 0 WHERE id = ?1", [id])?;
        Ok(())
    }

    pub fn move_and_remove_warehouse(
        &self,
        source_id: i64,
        target_id: i64,
    ) -> Result<(), DatabaseError> {
        if source_id == target_id {
            return Err(DatabaseError::Validation(
                "alternate warehouse must be different".into(),
            ));
        }
        let transaction = self.connection.unchecked_transaction()?;
        let target_active = transaction.query_row(
            "SELECT active FROM warehouses WHERE id = ?1",
            [target_id],
            |row| row.get::<_, bool>(0),
        )?;
        if !target_active {
            return Err(DatabaseError::Validation(
                "alternate warehouse is inactive".into(),
            ));
        }
        let mut statement = transaction.prepare(
            "SELECT item_id, quantity_milliunits FROM stock_levels
             WHERE warehouse_id = ?1 AND quantity_milliunits > 0",
        )?;
        let stock = statement
            .query_map([source_id], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        drop(statement);
        for (item_id, quantity) in stock {
            let total_cost = transaction.query_row(
                "SELECT COALESCE(SUM((remaining_quantity_milliunits * unit_cost_minor) / 1000), 0)
                 FROM inventory_cost_layers
                 WHERE warehouse_id = ?1 AND item_id = ?2 AND remaining_quantity_milliunits > 0",
                params![source_id, item_id],
                |row| row.get::<_, i64>(0),
            )?;
            transaction.execute(
                "INSERT INTO stock_levels (warehouse_id, item_id, quantity_milliunits)
                 VALUES (?1, ?2, ?3)
                 ON CONFLICT(warehouse_id, item_id) DO UPDATE SET
                    quantity_milliunits = quantity_milliunits + excluded.quantity_milliunits,
                    updated_at = CURRENT_TIMESTAMP",
                params![target_id, item_id, quantity],
            )?;
            for (warehouse_id, kind, increases) in [
                (source_id, "transfer_out", false),
                (target_id, "transfer_in", true),
            ] {
                transaction.execute(
                    "INSERT INTO inventory_movements
                     (warehouse_id, item_id, kind, quantity_milliunits, increases_stock,
                      occurred_on, reference, total_cost_minor)
                     VALUES (?1, ?2, ?3, ?4, ?5, date('now'), 'warehouse removal transfer', ?6)",
                    params![warehouse_id, item_id, kind, quantity, increases, total_cost],
                )?;
            }
        }
        transaction.execute(
            "UPDATE inventory_cost_layers SET warehouse_id = ?1
             WHERE warehouse_id = ?2 AND remaining_quantity_milliunits > 0",
            params![target_id, source_id],
        )?;
        transaction.execute(
            "DELETE FROM stock_levels WHERE warehouse_id = ?1",
            [source_id],
        )?;
        transaction.execute(
            "UPDATE warehouses SET active = 0 WHERE id = ?1",
            [source_id],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn stock_records(&self, search: &str) -> Result<Vec<StockRecord>, DatabaseError> {
        let pattern = format!("%{}%", search.trim());
        let mut statement = self.connection.prepare(
            "SELECT l.id, l.warehouse_id, w.name, l.item_id, i.name, COALESCE(i.sku, ''), i.unit,
                    l.remaining_quantity_milliunits, l.acquired_quantity_milliunits,
                    (l.remaining_quantity_milliunits * l.unit_cost_minor) / 1000,
                    l.unit_cost_minor
             FROM inventory_cost_layers l
             JOIN warehouses w ON w.id = l.warehouse_id
             JOIN catalog_items i ON i.id = l.item_id
             WHERE w.active = 1 AND i.active = 1
               AND l.archived_at IS NULL
               AND l.remaining_quantity_milliunits > 0
               AND (i.name LIKE ?1 OR COALESCE(i.sku, '') LIKE ?1 OR w.name LIKE ?1)
             ORDER BY i.name COLLATE NOCASE, w.name COLLATE NOCASE, l.acquired_on, l.id",
        )?;
        Ok(statement
            .query_map([pattern], |row| {
                Ok(StockRecord {
                    cost_layer_id: row.get(0)?,
                    warehouse_id: row.get(1)?,
                    warehouse_name: row.get(2)?,
                    item_id: row.get(3)?,
                    item_name: row.get(4)?,
                    sku: row.get(5)?,
                    unit: row.get(6)?,
                    quantity_milliunits: row.get(7)?,
                    acquired_quantity_milliunits: row.get(8)?,
                    inventory_value_minor: row.get(9)?,
                    unit_cost_minor: row.get(10)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?)
    }

    pub fn update_cost_layer(
        &self,
        layer_id: i64,
        target_warehouse_id: i64,
        quantity_milliunits: i64,
        unit_cost_minor: i64,
        reference: Option<&str>,
    ) -> Result<(), DatabaseError> {
        if quantity_milliunits < 0 || unit_cost_minor < 0 {
            return Err(DatabaseError::Validation(
                "invalid inventory layer values".into(),
            ));
        }
        let transaction = self.connection.unchecked_transaction()?;
        let (warehouse_id, item_id, acquired, previous_quantity, previous_cost) = transaction
            .query_row(
                "SELECT warehouse_id, item_id, acquired_quantity_milliunits,
                        remaining_quantity_milliunits, unit_cost_minor
                 FROM inventory_cost_layers WHERE id = ?1",
                [layer_id],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, i64>(3)?,
                        row.get::<_, i64>(4)?,
                    ))
                },
            )?;
        let delta = quantity_milliunits - previous_quantity;
        let acquired_quantity = if quantity_milliunits == 0 {
            acquired
        } else {
            acquired
                .checked_sub(previous_quantity)
                .and_then(|consumed| consumed.checked_add(quantity_milliunits))
                .ok_or_else(|| DatabaseError::Validation("stock quantity overflow".into()))?
        };
        let target_active = transaction.query_row(
            "SELECT active FROM warehouses WHERE id = ?1",
            [target_warehouse_id],
            |row| row.get::<_, bool>(0),
        )?;
        if !target_active {
            return Err(DatabaseError::Validation("warehouse is inactive".into()));
        }
        let current_stock = transaction.query_row(
            "SELECT quantity_milliunits FROM stock_levels
             WHERE warehouse_id = ?1 AND item_id = ?2",
            params![warehouse_id, item_id],
            |row| row.get::<_, i64>(0),
        )?;
        if warehouse_id == target_warehouse_id {
            let next_stock = current_stock
                .checked_add(delta)
                .ok_or_else(|| DatabaseError::Validation("stock quantity overflow".into()))?;
            if next_stock < 0 {
                return Err(DatabaseError::Validation("stock cannot be negative".into()));
            }
            transaction.execute(
                "UPDATE stock_levels SET quantity_milliunits = ?1, updated_at = CURRENT_TIMESTAMP
                 WHERE warehouse_id = ?2 AND item_id = ?3",
                params![next_stock, warehouse_id, item_id],
            )?;
        } else {
            let source_next = current_stock - previous_quantity;
            if source_next < 0 {
                return Err(DatabaseError::Validation("stock cannot be negative".into()));
            }
            transaction.execute(
                "UPDATE stock_levels SET quantity_milliunits = ?1, updated_at = CURRENT_TIMESTAMP
                 WHERE warehouse_id = ?2 AND item_id = ?3",
                params![source_next, warehouse_id, item_id],
            )?;
            transaction.execute(
                "INSERT INTO stock_levels (warehouse_id, item_id, quantity_milliunits)
                 VALUES (?1, ?2, ?3)
                 ON CONFLICT(warehouse_id, item_id) DO UPDATE SET
                    quantity_milliunits = quantity_milliunits + excluded.quantity_milliunits,
                    updated_at = CURRENT_TIMESTAMP",
                params![target_warehouse_id, item_id, quantity_milliunits],
            )?;
            let transfer_cost = previous_cost
                .checked_mul(previous_quantity)
                .ok_or_else(|| DatabaseError::Validation("inventory cost overflow".into()))?
                / 1_000;
            for (movement_warehouse, kind, increases) in [
                (warehouse_id, "transfer_out", false),
                (target_warehouse_id, "transfer_in", true),
            ] {
                transaction.execute(
                    "INSERT INTO inventory_movements
                     (warehouse_id, item_id, kind, quantity_milliunits, increases_stock,
                      occurred_on, reference, unit_cost_minor, total_cost_minor)
                     VALUES (?1, ?2, ?3, ?4, ?5, date('now'), ?6, ?7, ?8)",
                    params![
                        movement_warehouse,
                        item_id,
                        kind,
                        previous_quantity,
                        increases,
                        reference,
                        previous_cost,
                        transfer_cost
                    ],
                )?;
            }
        }
        transaction.execute(
            "UPDATE inventory_cost_layers
             SET acquired_quantity_milliunits = ?1,
                 remaining_quantity_milliunits = ?2,
                 unit_cost_minor = ?3,
                 warehouse_id = ?4
             WHERE id = ?5",
            params![
                acquired_quantity,
                quantity_milliunits,
                unit_cost_minor,
                target_warehouse_id,
                layer_id
            ],
        )?;
        transaction.execute(
            "INSERT INTO inventory_cost_layer_revisions
             (cost_layer_id, previous_quantity_milliunits, new_quantity_milliunits,
              previous_unit_cost_minor, new_unit_cost_minor, reference,
              previous_warehouse_id, new_warehouse_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                layer_id,
                previous_quantity,
                quantity_milliunits,
                previous_cost,
                unit_cost_minor,
                reference,
                warehouse_id,
                target_warehouse_id
            ],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn remove_cost_layer(&self, layer_id: i64) -> Result<(), DatabaseError> {
        let (warehouse_id, unit_cost_minor) = self.connection.query_row(
            "SELECT warehouse_id, unit_cost_minor FROM inventory_cost_layers WHERE id = ?1",
            [layer_id],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
        )?;
        self.update_cost_layer(
            layer_id,
            warehouse_id,
            0,
            unit_cost_minor,
            Some("removed from inventory"),
        )?;
        let transaction = self.connection.unchecked_transaction()?;
        transaction.execute(
            "UPDATE inventory_cost_layers
             SET archived_at = CURRENT_TIMESTAMP
             WHERE id = ?1",
            [layer_id],
        )?;
        transaction.execute(
            "UPDATE inventory_movements
             SET visible_in_history = 0
             WHERE id = (
                 SELECT source_movement_id FROM inventory_cost_layers WHERE id = ?1
             )",
            [layer_id],
        )?;
        transaction.execute(
            "UPDATE inventory_movements
             SET visible_in_history = EXISTS(
                 SELECT 1
                 FROM inventory_cost_allocations a
                 JOIN inventory_cost_layers l ON l.id = a.cost_layer_id
                 WHERE a.movement_id = inventory_movements.id
                   AND l.archived_at IS NULL
             )
             WHERE id IN (
                 SELECT movement_id FROM inventory_cost_allocations WHERE cost_layer_id = ?1
             )",
            [layer_id],
        )?;
        transaction.execute(
            "UPDATE inventory_movements
             SET visible_in_history = 0
             WHERE item_id = (SELECT item_id FROM inventory_cost_layers WHERE id = ?1)
               AND NOT EXISTS(
                   SELECT 1 FROM inventory_cost_layers l
                   WHERE l.item_id = inventory_movements.item_id
                     AND l.archived_at IS NULL
                     AND l.remaining_quantity_milliunits > 0
               )",
            [layer_id],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn inventory_products(
        &self,
        search: &str,
    ) -> Result<Vec<(i64, String, String)>, DatabaseError> {
        let pattern = format!("%{}%", search.trim());
        let mut statement = self.connection.prepare(
            "SELECT id, name, COALESCE(sku, '') FROM catalog_items
             WHERE active = 1 AND kind = 'product' AND track_inventory = 1
               AND (name LIKE ?1 OR COALESCE(sku, '') LIKE ?1)
             ORDER BY name COLLATE NOCASE LIMIT 50",
        )?;
        Ok(statement
            .query_map([pattern], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
            .collect::<Result<Vec<_>, _>>()?)
    }

    pub fn movement_records(&self) -> Result<Vec<MovementRecord>, DatabaseError> {
        let mut statement = self.connection.prepare(
            "SELECT i.name, w.name, m.quantity_milliunits, m.increases_stock,
                    COALESCE(m.reference, ''), m.occurred_on,
                    m.unit_cost_minor, m.total_cost_minor
             FROM inventory_movements m
             JOIN catalog_items i ON i.id = m.item_id
             JOIN warehouses w ON w.id = m.warehouse_id
             WHERE m.visible_in_history = 1
             ORDER BY m.id DESC LIMIT 250",
        )?;
        Ok(statement
            .query_map([], |row| {
                Ok(MovementRecord {
                    item_name: row.get(0)?,
                    warehouse_name: row.get(1)?,
                    quantity_milliunits: row.get(2)?,
                    increases_stock: row.get(3)?,
                    reference: row.get(4)?,
                    occurred_on: row.get(5)?,
                    unit_cost_minor: row.get(6)?,
                    total_cost_minor: row.get(7)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?)
    }

    pub fn set_stock_level(
        &self,
        warehouse_id: i64,
        item_id: i64,
        quantity_milliunits: i64,
        unit_cost_minor: Option<i64>,
        reference: Option<&str>,
    ) -> Result<(), DatabaseError> {
        if quantity_milliunits < 0 {
            return Err(DatabaseError::Validation("stock cannot be negative".into()));
        }
        let current = self.connection.query_row(
            "SELECT COALESCE((SELECT quantity_milliunits FROM stock_levels
             WHERE warehouse_id = ?1 AND item_id = ?2), 0)",
            params![warehouse_id, item_id],
            |row| row.get::<_, i64>(0),
        )?;
        if current == quantity_milliunits {
            return Ok(());
        }
        self.record_inventory_movement(&InventoryMovementDraft {
            warehouse_id,
            item_id,
            cost_layer_id: None,
            quantity_milliunits: (quantity_milliunits - current).abs(),
            increases_stock: quantity_milliunits > current,
            unit_cost_minor,
            reference: reference.map(str::to_owned),
        })
    }

    pub fn record_inventory_movement(
        &self,
        draft: &InventoryMovementDraft,
    ) -> Result<(), DatabaseError> {
        if draft.quantity_milliunits <= 0 {
            return Err(DatabaseError::Validation(
                "quantity must be positive".into(),
            ));
        }
        if draft.increases_stock && draft.unit_cost_minor.is_none() {
            return Err(DatabaseError::Validation(
                "purchase cost is required for incoming stock".into(),
            ));
        }
        if draft.increases_stock && draft.cost_layer_id.is_some() {
            return Err(DatabaseError::Validation(
                "incoming stock cannot target an existing layer".into(),
            ));
        }
        let transaction = self.connection.unchecked_transaction()?;
        let current = transaction.query_row(
            "SELECT COALESCE((SELECT quantity_milliunits FROM stock_levels
             WHERE warehouse_id = ?1 AND item_id = ?2), 0)",
            params![draft.warehouse_id, draft.item_id],
            |row| row.get::<_, i64>(0),
        )?;
        let signed = if draft.increases_stock {
            draft.quantity_milliunits
        } else {
            -draft.quantity_milliunits
        };
        let next = current
            .checked_add(signed)
            .ok_or_else(|| DatabaseError::Validation("stock quantity overflow".into()))?;
        if next < 0 {
            return Err(DatabaseError::Validation("insufficient stock".into()));
        }
        transaction.execute(
            "INSERT INTO stock_levels (warehouse_id, item_id, quantity_milliunits)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(warehouse_id, item_id) DO UPDATE
             SET quantity_milliunits = excluded.quantity_milliunits, updated_at = CURRENT_TIMESTAMP",
            params![draft.warehouse_id, draft.item_id, next],
        )?;
        let inbound_total = draft
            .unit_cost_minor
            .and_then(|cost| cost.checked_mul(draft.quantity_milliunits))
            .map(|value| value / 1_000);
        transaction.execute(
            "INSERT INTO inventory_movements
             (warehouse_id, item_id, kind, quantity_milliunits, increases_stock, occurred_on, reference,
              unit_cost_minor, total_cost_minor)
             VALUES (?1, ?2, ?3, ?4, ?5, date('now'), ?6, ?7, ?8)",
            params![
                draft.warehouse_id,
                draft.item_id,
                if draft.increases_stock { "purchase" } else { "sale" },
                draft.quantity_milliunits,
                draft.increases_stock,
                draft.reference,
                draft.unit_cost_minor,
                inbound_total,
            ],
        )?;
        let movement_id = transaction.last_insert_rowid();
        if draft.increases_stock {
            transaction.execute(
                "INSERT INTO inventory_cost_layers
                 (warehouse_id, item_id, source_movement_id, acquired_quantity_milliunits,
                  remaining_quantity_milliunits, unit_cost_minor, acquired_on)
                 VALUES (?1, ?2, ?3, ?4, ?4, ?5, date('now'))",
                params![
                    draft.warehouse_id,
                    draft.item_id,
                    movement_id,
                    draft.quantity_milliunits,
                    draft.unit_cost_minor.unwrap_or_default(),
                ],
            )?;
        } else {
            let mut remaining = draft.quantity_milliunits;
            let mut total_cost = 0_i64;
            let mut statement = transaction.prepare(
                "SELECT id, remaining_quantity_milliunits, unit_cost_minor
                 FROM inventory_cost_layers
                 WHERE warehouse_id = ?1 AND item_id = ?2 AND remaining_quantity_milliunits > 0
                   AND archived_at IS NULL
                   AND (?3 IS NULL OR id = ?3)
                 ORDER BY acquired_on, id",
            )?;
            let layers = statement
                .query_map(
                    params![draft.warehouse_id, draft.item_id, draft.cost_layer_id],
                    |row| {
                        Ok((
                            row.get::<_, i64>(0)?,
                            row.get::<_, i64>(1)?,
                            row.get::<_, i64>(2)?,
                        ))
                    },
                )?
                .collect::<Result<Vec<_>, _>>()?;
            drop(statement);
            for (layer_id, available, unit_cost) in layers {
                if remaining == 0 {
                    break;
                }
                let used = remaining.min(available);
                let allocated = unit_cost
                    .checked_mul(used)
                    .ok_or_else(|| DatabaseError::Validation("inventory cost overflow".into()))?
                    / 1_000;
                total_cost = total_cost
                    .checked_add(allocated)
                    .ok_or_else(|| DatabaseError::Validation("inventory cost overflow".into()))?;
                transaction.execute(
                    "UPDATE inventory_cost_layers
                     SET remaining_quantity_milliunits = remaining_quantity_milliunits - ?1
                     WHERE id = ?2",
                    params![used, layer_id],
                )?;
                transaction.execute(
                    "INSERT INTO inventory_cost_allocations
                     (movement_id, cost_layer_id, quantity_milliunits, unit_cost_minor, allocated_cost_minor)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![movement_id, layer_id, used, unit_cost, allocated],
                )?;
                remaining -= used;
            }
            if remaining != 0 {
                return Err(DatabaseError::Validation(
                    "inventory cost layers are incomplete".into(),
                ));
            }
            transaction.execute(
                "UPDATE inventory_movements
                 SET total_cost_minor = ?1,
                     unit_cost_minor = CASE WHEN quantity_milliunits = 0 THEN 0
                         ELSE (?1 * 1000) / quantity_milliunits END
                 WHERE id = ?2",
                params![total_cost, movement_id],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }
}
