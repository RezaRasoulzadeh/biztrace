// src/models/customer.rs

use super::ModelError;

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
            active: true,
        })
    }
}
