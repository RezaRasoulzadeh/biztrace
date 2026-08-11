// src/models/catalog.rs

use super::{ModelError, Money};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CatalogItemId(pub i64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemKind {
    Product,
    Service,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unit {
    Each,
    Kilogram,
    Gram,
    Liter,
    Meter,
    Hour,
    Session,
    Custom,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogItem {
    pub id: CatalogItemId,
    pub kind: ItemKind,
    pub name: String,
    pub sku: Option<String>,
    pub description: Option<String>,
    pub unit: Unit,
    pub sale_price: Money,
    pub cost_price: Option<Money>,
    pub tax_basis_points: u16,
    pub track_inventory: bool,
    pub active: bool,
}

impl CatalogItem {
    pub fn new(
        id: CatalogItemId,
        kind: ItemKind,
        name: impl Into<String>,
        unit: Unit,
        sale_price: Money,
    ) -> Result<Self, ModelError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(ModelError::EmptyField("catalog_item.name"));
        }
        Ok(Self {
            id,
            kind,
            name,
            sku: None,
            description: None,
            unit,
            sale_price,
            cost_price: None,
            tax_basis_points: 0,
            track_inventory: kind == ItemKind::Product,
            active: true,
        })
    }
}
