// src/models/fund.rs

use super::{Currency, Date, InvoiceId, Money, UserId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FundAccountId(pub i64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FundTransactionId(pub i64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransactionKind {
    Income,
    Expense,
    Transfer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FundAccount {
    pub id: FundAccountId,
    pub name: String,
    pub currency: Currency,
    pub opening_balance: Money,
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FundTransaction {
    pub id: FundTransactionId,
    pub account_id: FundAccountId,
    pub transfer_account_id: Option<FundAccountId>,
    pub kind: TransactionKind,
    pub amount: Money,
    pub category: String,
    pub occurred_on: Date,
    pub invoice_id: Option<InvoiceId>,
    pub description: Option<String>,
    pub created_by: UserId,
}
