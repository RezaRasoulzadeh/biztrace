// src/models/customer.rs

use super::{Currency, ModelError, Money};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CustomerId(pub i64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CustomerKind {
    Individual,
    Business,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Customer {
    pub id: CustomerId,
    pub kind: CustomerKind,
    pub name: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub tax_id: Option<String>,
    pub notes: Option<String>,
    pub balance: Money,
    pub active: bool,
}

impl Customer {
    pub fn new(
        id: CustomerId,
        kind: CustomerKind,
        name: impl Into<String>,
    ) -> Result<Self, ModelError> {
        let name = name.into();
        if name.trim().is_empty() {
            return Err(ModelError::EmptyField("customer.name"));
        }
        Ok(Self {
            id,
            kind,
            name,
            phone: None,
            email: None,
            address: None,
            tax_id: None,
            notes: None,
            balance: Money::zero(Currency::Irr),
            active: true,
        })
    }

    pub fn is_debit(&self) -> bool {
        self.balance.minor_units > 0
    }

    pub fn is_credit(&self) -> bool {
        self.balance.minor_units < 0
    }
}
