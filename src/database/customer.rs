// src/database/customer.rs

use rusqlite::{OptionalExtension, params};

use super::{Database, DatabaseError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomerRecord {
    pub id: i64,
    pub kind: String,
    pub name: String,
    pub phone: String,
    pub email: String,
    pub address: String,
    pub balance_minor: i64,
    pub currency: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomerDraft {
    pub id: Option<i64>,
    pub kind: String,
    pub name: String,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
}

impl Database {
    pub fn adjust_customer_balance(
        &self,
        id: i64,
        amount_minor: i64,
        note: Option<&str>,
    ) -> Result<i64, DatabaseError> {
        if amount_minor == 0 {
            return Err(DatabaseError::Validation(
                "balance adjustment cannot be zero".into(),
            ));
        }
        let current = self
            .customer(id)?
            .ok_or_else(|| DatabaseError::Validation("customer not found".into()))?
            .balance_minor;
        let updated = current
            .checked_add(amount_minor)
            .ok_or_else(|| DatabaseError::Validation("balance overflow".into()))?;
        let transaction = self.connection.unchecked_transaction()?;
        transaction.execute("UPDATE customers SET balance_minor = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2 AND active = 1", params![updated, id])?;
        transaction.execute("INSERT INTO customer_balance_entries (customer_id, amount_minor, kind, note) VALUES (?1, ?2, ?3, ?4)", params![id, amount_minor, if amount_minor > 0 { "debit" } else { "credit" }, note])?;
        transaction.commit()?;
        Ok(updated)
    }

    pub fn settle_customer_balance(&self, id: i64) -> Result<(), DatabaseError> {
        let current = self
            .customer(id)?
            .ok_or_else(|| DatabaseError::Validation("customer not found".into()))?
            .balance_minor;
        if current == 0 {
            return Ok(());
        }
        let transaction = self.connection.unchecked_transaction()?;
        transaction.execute("UPDATE customers SET balance_minor = 0, updated_at = CURRENT_TIMESTAMP WHERE id = ?1 AND active = 1", [id])?;
        transaction.execute("INSERT INTO customer_balance_entries (customer_id, amount_minor, kind) VALUES (?1, ?2, 'settlement')", params![id, -current])?;
        transaction.commit()?;
        Ok(())
    }
    pub fn customers(&self, search: &str) -> Result<Vec<CustomerRecord>, DatabaseError> {
        let pattern = format!("%{}%", search.trim());
        let mut statement = self.connection.prepare(
            "SELECT id, kind, name, COALESCE(phone, ''), COALESCE(email, ''), COALESCE(address, ''),
                    balance_minor, currency
             FROM customers
             WHERE active = 1 AND (
                name LIKE ?1 OR COALESCE(phone, '') LIKE ?1 OR COALESCE(email, '') LIKE ?1
             )
             ORDER BY updated_at DESC, id DESC",
        )?;
        let records = statement
            .query_map([pattern], |row| {
                Ok(CustomerRecord {
                    id: row.get(0)?,
                    kind: row.get(1)?,
                    name: row.get(2)?,
                    phone: row.get(3)?,
                    email: row.get(4)?,
                    address: row.get(5)?,
                    balance_minor: row.get(6)?,
                    currency: row.get(7)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(records)
    }

    pub fn save_customer(&self, draft: &CustomerDraft) -> Result<i64, DatabaseError> {
        if draft.name.trim().is_empty() {
            return Err(DatabaseError::Validation(
                "customer name is required".into(),
            ));
        }
        if !matches!(draft.kind.as_str(), "individual" | "business") {
            return Err(DatabaseError::Validation("invalid customer kind".into()));
        }
        if let Some(id) = draft.id {
            self.connection.execute(
                "UPDATE customers
                 SET kind = ?1, name = ?2, phone = ?3, email = ?4, address = ?5,
                     updated_at = CURRENT_TIMESTAMP
                 WHERE id = ?6 AND active = 1",
                params![
                    draft.kind,
                    draft.name,
                    draft.phone,
                    draft.email,
                    draft.address,
                    id
                ],
            )?;
            return Ok(id);
        }
        self.connection.execute(
            "INSERT INTO customers (kind, name, phone, email, address, balance_minor, currency)
             VALUES (?1, ?2, ?3, ?4, ?5, 0, 'IRR')",
            params![
                draft.kind,
                draft.name,
                draft.phone,
                draft.email,
                draft.address
            ],
        )?;
        Ok(self.connection.last_insert_rowid())
    }

    pub fn customer(&self, id: i64) -> Result<Option<CustomerRecord>, DatabaseError> {
        self.connection
            .query_row(
                "SELECT id, kind, name, COALESCE(phone, ''), COALESCE(email, ''), COALESCE(address, ''),
                        balance_minor, currency
                 FROM customers WHERE id = ?1 AND active = 1",
                [id],
                |row| {
                    Ok(CustomerRecord {
                        id: row.get(0)?,
                        kind: row.get(1)?,
                        name: row.get(2)?,
                        phone: row.get(3)?,
                        email: row.get(4)?,
                        address: row.get(5)?,
                        balance_minor: row.get(6)?,
                        currency: row.get(7)?,
                    })
                },
            )
            .optional()
            .map_err(DatabaseError::from)
    }

    pub fn remove_customer(&self, id: i64) -> Result<(), DatabaseError> {
        let has_invoices = self.connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM invoices WHERE customer_id = ?1)",
            [id],
            |row| row.get::<_, bool>(0),
        )?;
        if has_invoices {
            return Err(DatabaseError::Validation(
                "customer still has invoices on record".into(),
            ));
        }
        self.connection.execute(
            "UPDATE customers SET active = 0, updated_at = CURRENT_TIMESTAMP
             WHERE id = ?1 AND active = 1",
            [id],
        )?;
        Ok(())
    }
}
