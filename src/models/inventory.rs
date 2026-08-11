// src/models/inventory.rs

use super::{CatalogItemId, Date, Quantity, UserId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WarehouseId(pub i64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InventoryMovementId(pub i64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MovementKind {
    Opening,
    Purchase,
    Sale,
    CustomerReturn,
    SupplierReturn,
    Adjustment,
    TransferIn,
    TransferOut,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StockLevel {
    pub warehouse_id: WarehouseId,
    pub item_id: CatalogItemId,
    pub quantity_milliunits: i64,
    pub reorder_point_milliunits: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InventoryMovement {
    pub id: InventoryMovementId,
    pub warehouse_id: WarehouseId,
    pub item_id: CatalogItemId,
    pub kind: MovementKind,
    pub quantity: Quantity,
    pub increases_stock: bool,
    pub occurred_on: Date,
    pub reference: Option<String>,
    pub note: Option<String>,
    pub created_by: UserId,
}

impl InventoryMovement {
    pub const fn signed_milliunits(&self) -> i64 {
        if self.increases_stock {
            self.quantity.milliunits()
        } else {
            -self.quantity.milliunits()
        }
    }
}
