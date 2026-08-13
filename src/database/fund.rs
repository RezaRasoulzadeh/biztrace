use super::{Database, DatabaseError};
use rusqlite::params;

#[derive(Debug, Clone)]
pub struct FundAccountRecord {
    pub id: i64,
    pub kind: String,
    pub name: String,
    pub account_number: String,
    pub opening_balance_minor: i64,
    pub balance_minor: i64,
}
#[derive(Debug, Clone)]
pub struct FundAccountDraft {
    pub id: Option<i64>,
    pub kind: String,
    pub name: String,
    pub account_number: Option<String>,
    pub opening_balance_minor: i64,
}
#[derive(Debug, Clone)]
pub struct FundTransactionRecord {
    pub id: i64,
    pub kind: String,
    pub account_name: String,
    pub transfer_account_name: String,
    pub amount_minor: i64,
    pub category: String,
    pub occurred_on: String,
    pub reference: String,
    pub description: String,
}
#[derive(Debug, Clone)]
pub struct FundTransactionDraft {
    pub account_id: i64,
    pub transfer_account_id: Option<i64>,
    pub kind: String,
    pub amount_minor: i64,
    pub category: String,
    pub occurred_on: String,
    pub reference: Option<String>,
    pub description: Option<String>,
}
#[derive(Debug, Clone)]
pub struct FundCheckRecord {
    pub id: i64,
    pub schedule_type: String,
    pub direction: String,
    pub account_name: String,
    pub party_name: String,
    pub check_number: String,
    pub bank_name: String,
    pub amount_minor: i64,
    pub due_on: String,
    pub status: String,
    pub note: String,
}
#[derive(Debug, Clone)]
pub struct FundCheckDraft {
    pub schedule_type: String,
    pub direction: String,
    pub account_id: i64,
    pub party_name: String,
    pub check_number: String,
    pub bank_name: Option<String>,
    pub amount_minor: i64,
    pub due_on: String,
    pub note: Option<String>,
}

impl Database {
    pub fn fund_accounts(&self) -> Result<Vec<FundAccountRecord>, DatabaseError> {
        let mut s=self.connection.prepare("SELECT a.id,a.kind,a.name,COALESCE(a.account_number,''),a.opening_balance_minor,a.opening_balance_minor+COALESCE(SUM(CASE WHEN t.kind='income' THEN t.amount_minor WHEN t.kind='expense' THEN -t.amount_minor WHEN t.kind='transfer' AND t.account_id=a.id THEN -t.amount_minor WHEN t.kind='transfer' AND t.transfer_account_id=a.id THEN t.amount_minor ELSE 0 END),0) FROM fund_accounts a LEFT JOIN fund_transactions t ON t.account_id=a.id OR t.transfer_account_id=a.id WHERE a.active=1 GROUP BY a.id ORDER BY a.updated_at DESC,a.id DESC")?;
        Ok(s.query_map([], |r| {
            Ok(FundAccountRecord {
                id: r.get(0)?,
                kind: r.get(1)?,
                name: r.get(2)?,
                account_number: r.get(3)?,
                opening_balance_minor: r.get(4)?,
                balance_minor: r.get(5)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?)
    }
    pub fn save_fund_account(&self, d: &FundAccountDraft) -> Result<i64, DatabaseError> {
        if d.name.trim().is_empty() || d.opening_balance_minor < 0 {
            return Err(DatabaseError::Validation("invalid account".into()));
        }
        if d.kind == "cash" && d.account_number.is_some() {
            return Err(DatabaseError::Validation(
                "cash account cannot have an account number".into(),
            ));
        }
        if !matches!(d.kind.as_str(), "cash" | "bank" | "card" | "other") {
            return Err(DatabaseError::Validation("invalid account kind".into()));
        }
        if let Some(id) = d.id {
            self.connection.execute("UPDATE fund_accounts SET kind=?1,name=?2,account_number=?3,opening_balance_minor=?4,updated_at=CURRENT_TIMESTAMP WHERE id=?5 AND active=1",params![d.kind,d.name,d.account_number,d.opening_balance_minor,id])?;
            Ok(id)
        } else {
            self.connection.execute("INSERT INTO fund_accounts(kind,name,account_number,opening_balance_minor,currency)VALUES(?1,?2,?3,?4,'IRR')",params![d.kind,d.name,d.account_number,d.opening_balance_minor])?;
            Ok(self.connection.last_insert_rowid())
        }
    }
    pub fn remove_fund_account(&self, id: i64) -> Result<(), DatabaseError> {
        let used:bool=self.connection.query_row("SELECT EXISTS(SELECT 1 FROM fund_transactions WHERE account_id=?1 OR transfer_account_id=?1) OR EXISTS(SELECT 1 FROM fund_checks WHERE account_id=?1)",[id],|r|r.get(0))?;
        if used {
            return Err(DatabaseError::Validation("account is in use".into()));
        }
        self.connection.execute(
            "UPDATE fund_accounts SET active=0,updated_at=CURRENT_TIMESTAMP WHERE id=?1",
            [id],
        )?;
        Ok(())
    }
    pub fn fund_transactions(
        &self,
        search: &str,
    ) -> Result<Vec<FundTransactionRecord>, DatabaseError> {
        let p = format!("%{}%", search.trim());
        let mut s=self.connection.prepare("SELECT t.id,t.kind,a.name,COALESCE(ta.name,''),t.amount_minor,t.category,t.occurred_on,COALESCE(t.reference,''),COALESCE(t.description,'') FROM fund_transactions t JOIN fund_accounts a ON a.id=t.account_id LEFT JOIN fund_accounts ta ON ta.id=t.transfer_account_id WHERE a.name LIKE ?1 OR COALESCE(ta.name,'') LIKE ?1 OR COALESCE(t.reference,'') LIKE ?1 OR COALESCE(t.description,'') LIKE ?1 ORDER BY t.occurred_on DESC,t.id DESC")?;
        Ok(s.query_map([p], |r| {
            Ok(FundTransactionRecord {
                id: r.get(0)?,
                kind: r.get(1)?,
                account_name: r.get(2)?,
                transfer_account_name: r.get(3)?,
                amount_minor: r.get(4)?,
                category: r.get(5)?,
                occurred_on: r.get(6)?,
                reference: r.get(7)?,
                description: r.get(8)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?)
    }
    pub fn save_fund_transaction(&self, d: &FundTransactionDraft) -> Result<i64, DatabaseError> {
        if d.amount_minor <= 0 || !valid_iso_date(&d.occurred_on) {
            return Err(DatabaseError::Validation("invalid transaction".into()));
        }
        validate_transaction_accounts(d)?;
        self.connection.execute("INSERT INTO fund_transactions(account_id,transfer_account_id,kind,amount_minor,currency,category,occurred_on,reference,description,created_by)VALUES(?1,?2,?3,?4,'IRR',?5,?6,?7,?8,(SELECT id FROM users ORDER BY id LIMIT 1))",params![d.account_id,d.transfer_account_id,d.kind,d.amount_minor,d.category,d.occurred_on,d.reference,d.description])?;
        Ok(self.connection.last_insert_rowid())
    }
    pub fn remove_fund_transaction(&self, id: i64) -> Result<(), DatabaseError> {
        self.connection
            .execute("DELETE FROM fund_transactions WHERE id=?1", [id])?;
        Ok(())
    }
    pub fn update_fund_transaction(
        &self,
        id: i64,
        d: &FundTransactionDraft,
    ) -> Result<(), DatabaseError> {
        if d.amount_minor <= 0 || !valid_iso_date(&d.occurred_on) {
            return Err(DatabaseError::Validation("invalid transaction".into()));
        }
        validate_transaction_accounts(d)?;
        self.connection.execute("UPDATE fund_transactions SET account_id=?1,transfer_account_id=?2,kind=?3,amount_minor=?4,category=?5,occurred_on=?6,reference=?7,description=?8,updated_at=CURRENT_TIMESTAMP WHERE id=?9",params![d.account_id,d.transfer_account_id,d.kind,d.amount_minor,d.category,d.occurred_on,d.reference,d.description,id])?;
        Ok(())
    }
    pub fn fund_checks(&self, search: &str) -> Result<Vec<FundCheckRecord>, DatabaseError> {
        let p = format!("%{}%", search.trim());
        let mut s=self.connection.prepare("SELECT c.id,c.schedule_type,c.direction,a.name,c.party_name,CASE WHEN c.check_number LIKE 'AUTO-%' THEN '' ELSE c.check_number END,COALESCE(c.bank_name,''),c.amount_minor,c.due_on,c.status,COALESCE(c.note,'') FROM fund_checks c JOIN fund_accounts a ON a.id=c.account_id WHERE c.party_name LIKE ?1 OR c.check_number LIKE ?1 OR COALESCE(c.bank_name,'') LIKE ?1 OR COALESCE(c.note,'') LIKE ?1 ORDER BY CASE WHEN c.status='upcoming' THEN 0 ELSE 1 END,c.due_on,c.id DESC")?;
        Ok(s.query_map([p], |r| {
            Ok(FundCheckRecord {
                id: r.get(0)?,
                schedule_type: r.get(1)?,
                direction: r.get(2)?,
                account_name: r.get(3)?,
                party_name: r.get(4)?,
                check_number: r.get(5)?,
                bank_name: r.get(6)?,
                amount_minor: r.get(7)?,
                due_on: r.get(8)?,
                status: r.get(9)?,
                note: r.get(10)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?)
    }
    pub fn save_fund_check(&self, d: &FundCheckDraft) -> Result<i64, DatabaseError> {
        if d.amount_minor <= 0
            || d.party_name.trim().is_empty()
            || !valid_iso_date(&d.due_on)
            || !matches!(
                d.schedule_type.as_str(),
                "check" | "installment" | "scheduled"
            )
            || (d.schedule_type == "check" && d.check_number.trim().is_empty())
        {
            return Err(DatabaseError::Validation("invalid check".into()));
        }
        let account_kind: String = self.connection.query_row(
            "SELECT kind FROM fund_accounts WHERE id=?1 AND active=1",
            [d.account_id],
            |r| r.get(0),
        )?;
        if d.schedule_type == "check" && account_kind == "cash" {
            return Err(DatabaseError::Validation(
                "checks require a bank or card account".into(),
            ));
        }
        self.connection.execute("INSERT INTO fund_checks(schedule_type,direction,account_id,party_name,check_number,bank_name,amount_minor,due_on,note)VALUES(?1,?2,?3,?4,CASE WHEN trim(?5)='' THEN 'AUTO-'||lower(hex(randomblob(8))) ELSE ?5 END,?6,?7,?8,?9)",params![d.schedule_type,d.direction,d.account_id,d.party_name,d.check_number,d.bank_name,d.amount_minor,d.due_on,d.note])?;
        Ok(self.connection.last_insert_rowid())
    }
    pub fn set_fund_check_status(&self, id: i64, status: &str) -> Result<(), DatabaseError> {
        if !matches!(status, "upcoming" | "cleared" | "returned" | "cancelled") {
            return Err(DatabaseError::Validation("invalid status".into()));
        }
        let transaction = self.connection.unchecked_transaction()?;
        let (schedule_type, direction, account_id, amount, due_on, party, current) = transaction.query_row(
            "SELECT schedule_type,direction,account_id,amount_minor,due_on,party_name,status FROM fund_checks WHERE id=?1",
            [id], |r| Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,i64>(2)?,r.get::<_,i64>(3)?,r.get::<_,String>(4)?,r.get::<_,String>(5)?,r.get::<_,String>(6)?)))?;
        if current != "upcoming" {
            return Err(DatabaseError::Validation(
                "check is already finalized".into(),
            ));
        }
        transaction.execute(
            "UPDATE fund_checks SET status=?1,updated_at=CURRENT_TIMESTAMP WHERE id=?2",
            params![status, id],
        )?;
        if status == "cleared" {
            let label = match schedule_type.as_str() {
                "check" => "چک",
                "installment" => "قسط",
                _ => "پرداخت زمان‌بندی‌شده",
            };
            transaction.execute("INSERT INTO fund_transactions(account_id,kind,amount_minor,currency,category,occurred_on,reference,description) VALUES(?1,?2,?3,'IRR',?4,?5,?6,?7)",params![account_id,if direction=="incoming"{"income"}else{"expense"},amount,label,due_on,format!("CHECK-{id}"),format!("تسویه {label} {party}")])?;
        }
        transaction.commit()?;
        Ok(())
    }
    pub fn update_fund_check(&self, id: i64, d: &FundCheckDraft) -> Result<(), DatabaseError> {
        if d.amount_minor <= 0
            || d.party_name.trim().is_empty()
            || !valid_iso_date(&d.due_on)
            || !matches!(
                d.schedule_type.as_str(),
                "check" | "installment" | "scheduled"
            )
            || (d.schedule_type == "check" && d.check_number.trim().is_empty())
        {
            return Err(DatabaseError::Validation("invalid scheduled item".into()));
        }
        let transaction = self.connection.unchecked_transaction()?;
        let account_kind: String = transaction.query_row(
            "SELECT kind FROM fund_accounts WHERE id=?1 AND active=1",
            [d.account_id],
            |row| row.get(0),
        )?;
        if d.schedule_type == "check" && account_kind == "cash" {
            return Err(DatabaseError::Validation(
                "checks require a bank or card account".into(),
            ));
        }
        let status: String =
            transaction.query_row("SELECT status FROM fund_checks WHERE id=?1", [id], |r| {
                r.get(0)
            })?;
        transaction.execute("UPDATE fund_checks SET schedule_type=?1,direction=?2,account_id=?3,party_name=?4,check_number=CASE WHEN trim(?5)='' THEN 'AUTO-'||lower(hex(randomblob(8))) ELSE ?5 END,bank_name=?6,amount_minor=?7,due_on=?8,note=?9,updated_at=CURRENT_TIMESTAMP WHERE id=?10",params![d.schedule_type,d.direction,d.account_id,d.party_name,d.check_number,d.bank_name,d.amount_minor,d.due_on,d.note,id])?;
        if status == "cleared" {
            transaction.execute("UPDATE fund_transactions SET account_id=?1,kind=?2,amount_minor=?3,occurred_on=?4,description=?5,updated_at=CURRENT_TIMESTAMP WHERE reference=?6",params![d.account_id,if d.direction=="incoming"{"income"}else{"expense"},d.amount_minor,d.due_on,format!("تسویه چک {}",d.party_name),format!("CHECK-{id}")])?;
        }
        transaction.commit()?;
        Ok(())
    }
    pub fn remove_fund_check(&self, id: i64) -> Result<(), DatabaseError> {
        let transaction = self.connection.unchecked_transaction()?;
        transaction.execute(
            "DELETE FROM fund_transactions WHERE reference=?1",
            [format!("CHECK-{id}")],
        )?;
        transaction.execute("DELETE FROM fund_checks WHERE id=?1", [id])?;
        transaction.commit()?;
        Ok(())
    }
}

fn validate_transaction_accounts(d: &FundTransactionDraft) -> Result<(), DatabaseError> {
    match (d.kind.as_str(), d.transfer_account_id) {
        ("transfer", Some(target)) if target != d.account_id => Ok(()),
        ("income" | "expense", None) => Ok(()),
        ("transfer", _) => Err(DatabaseError::Validation(
            "source and destination accounts must be different".into(),
        )),
        _ => Err(DatabaseError::Validation("invalid transaction kind".into())),
    }
}

fn valid_iso_date(value: &str) -> bool {
    let parts = value.split('-').collect::<Vec<_>>();
    if parts.len() != 3 || parts[0].len() != 4 || parts[1].len() != 2 || parts[2].len() != 2 {
        return false;
    }
    let (Ok(year), Ok(month), Ok(day)) = (
        parts[0].parse::<i32>(),
        parts[1].parse::<u8>(),
        parts[2].parse::<u8>(),
    ) else {
        return false;
    };
    crate::models::Date::new(year, month, day).is_ok()
}
