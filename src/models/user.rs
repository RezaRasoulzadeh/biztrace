// src/models/user.rs

use super::ModelError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UserId(pub i64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserRole {
    Owner,
    Administrator,
    Manager,
    Sales,
    Accountant,
    Inventory,
    Staff,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserStatus {
    Active,
    Suspended,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct User {
    pub id: UserId,
    pub username: String,
    pub display_name: String,
    pub password_hash: String,
    pub role: UserRole,
    pub status: UserStatus,
}

impl User {
    pub fn new(
        id: UserId,
        username: impl Into<String>,
        display_name: impl Into<String>,
        password_hash: impl Into<String>,
        role: UserRole,
    ) -> Result<Self, ModelError> {
        let username = username.into();
        let display_name = display_name.into();
        let password_hash = password_hash.into();
        if username.trim().is_empty() {
            return Err(ModelError::EmptyField("username"));
        }
        if display_name.trim().is_empty() {
            return Err(ModelError::EmptyField("display_name"));
        }
        if password_hash.is_empty() {
            return Err(ModelError::EmptyField("password_hash"));
        }
        Ok(Self {
            id,
            username,
            display_name,
            password_hash,
            role,
            status: UserStatus::Active,
        })
    }
}
