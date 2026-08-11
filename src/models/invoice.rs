// src/models/invoice.rs

use super::{
    CatalogItemId, Currency, CustomerId, Date, ItemKind, ModelError, Money, Quantity, UserId,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InvoiceId(pub i64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvoiceStatus {
    Draft,
    Issued,
    PartiallyPaid,
    Paid,
    Voided,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvoiceLine {
    pub item_id: Option<CatalogItemId>,
    pub item_kind: ItemKind,
    pub description: String,
    pub quantity: Quantity,
    pub unit_price: Money,
    pub discount: Money,
    pub tax_basis_points: u16,
}

impl InvoiceLine {
    pub fn subtotal(&self) -> Result<Money, ModelError> {
        let multiplied = self
            .unit_price
            .minor_units
            .checked_mul(self.quantity.milliunits())
            .ok_or(ModelError::ArithmeticOverflow)?;
        let subtotal = multiplied / Quantity::SCALE;
        Money::new(subtotal, self.unit_price.currency).checked_sub(self.discount)
    }

    pub fn tax(&self) -> Result<Money, ModelError> {
        let subtotal = self.subtotal()?;
        let value = subtotal
            .minor_units
            .checked_mul(i64::from(self.tax_basis_points))
            .ok_or(ModelError::ArithmeticOverflow)?
            / 10_000;
        Ok(Money::new(value, subtotal.currency))
    }

    pub fn total(&self) -> Result<Money, ModelError> {
        self.subtotal()?.checked_add(self.tax()?)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invoice {
    pub id: InvoiceId,
    pub number: String,
    pub customer_id: Option<CustomerId>,
    pub status: InvoiceStatus,
    pub issued_on: Date,
    pub due_on: Option<Date>,
    pub currency: Currency,
    pub lines: Vec<InvoiceLine>,
    pub notes: Option<String>,
    pub created_by: UserId,
}

impl Invoice {
    pub fn total(&self) -> Result<Money, ModelError> {
        self.lines
            .iter()
            .try_fold(Money::zero(self.currency), |total, line| {
                total.checked_add(line.total()?)
            })
    }
}
