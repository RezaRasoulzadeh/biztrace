// src/models/mod.rs

mod catalog;
mod common;
mod customer;
mod fund;
mod inventory;
mod invoice;
mod user;

pub use catalog::{CatalogItem, CatalogItemId, ItemKind, Unit};
pub use common::{Currency, Date, ModelError, Money, Quantity};
pub use customer::{Customer, CustomerId, CustomerKind};
pub use fund::{
    CheckDirection, CheckStatus, FundAccount, FundAccountId, FundAccountKind, FundCheck,
    FundTransaction, FundTransactionId, TransactionKind,
};
pub use inventory::{
    InventoryMovement, InventoryMovementId, MovementKind, StockLevel, WarehouseId,
};
pub use invoice::{Invoice, InvoiceId, InvoiceLine, InvoiceStatus};
pub use user::{User, UserId, UserRole, UserStatus};
