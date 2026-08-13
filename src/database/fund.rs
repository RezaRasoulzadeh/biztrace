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
        let mut s=self.connection.prepare("SELECT t.id,t.kind,a.name,COALESCE(ta.name,''),t.amount_minor,t.category,t.occurred_on,COALESCE(t.reference,''),COALESCE(t.description,'') FROM fund_transactions t JOIN fund_accounts a ON a.id=t.account_id LEFT JOIN fund_accounts ta ON ta.id=t.transfer_account_id WHERE a.name LIKE ?1 OR t.category LIKE ?1 OR COALESCE(t.reference,'') LIKE ?1 ORDER BY t.occurred_on DESC,t.id DESC")?;
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
        if d.amount_minor <= 0 || d.category.trim().is_empty() || d.occurred_on.trim().is_empty() {
            return Err(DatabaseError::Validation("invalid transaction".into()));
        }
        self.connection.execute("INSERT INTO fund_transactions(account_id,transfer_account_id,kind,amount_minor,currency,category,occurred_on,reference,description,created_by)VALUES(?1,?2,?3,?4,'IRR',?5,?6,?7,?8,(SELECT id FROM users ORDER BY id LIMIT 1))",params![d.account_id,d.transfer_account_id,d.kind,d.amount_minor,d.category,d.occurred_on,d.reference,d.description])?;
        Ok(self.connection.last_insert_rowid())
    }
    pub fn remove_fund_transaction(&self, id: i64) -> Result<(), DatabaseError> {
        self.connection
            .execute("DELETE FROM fund_transactions WHERE id=?1", [id])?;
        Ok(())
    }
    pub fn fund_checks(&self, search: &str) -> Result<Vec<FundCheckRecord>, DatabaseError> {
        let p = format!("%{}%", search.trim());
        let mut s=self.connection.prepare("SELECT c.id,c.direction,a.name,c.party_name,c.check_number,COALESCE(c.bank_name,''),c.amount_minor,c.due_on,c.status,COALESCE(c.note,'') FROM fund_checks c JOIN fund_accounts a ON a.id=c.account_id WHERE c.party_name LIKE ?1 OR c.check_number LIKE ?1 OR COALESCE(c.bank_name,'') LIKE ?1 ORDER BY CASE WHEN c.status='upcoming' THEN 0 ELSE 1 END,c.due_on,c.id DESC")?;
        Ok(s.query_map([p], |r| {
            Ok(FundCheckRecord {
                id: r.get(0)?,
                direction: r.get(1)?,
                account_name: r.get(2)?,
                party_name: r.get(3)?,
                check_number: r.get(4)?,
                bank_name: r.get(5)?,
                amount_minor: r.get(6)?,
                due_on: r.get(7)?,
                status: r.get(8)?,
                note: r.get(9)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?)
    }
    pub fn save_fund_check(&self, d: &FundCheckDraft) -> Result<i64, DatabaseError> {
        if d.amount_minor <= 0
            || d.party_name.trim().is_empty()
            || d.check_number.trim().is_empty()
            || d.due_on.trim().is_empty()
        {
            return Err(DatabaseError::Validation("invalid check".into()));
        }
        self.connection.execute("INSERT INTO fund_checks(direction,account_id,party_name,check_number,bank_name,amount_minor,due_on,note)VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",params![d.direction,d.account_id,d.party_name,d.check_number,d.bank_name,d.amount_minor,d.due_on,d.note])?;
        Ok(self.connection.last_insert_rowid())
    }
    pub fn set_fund_check_status(&self, id: i64, status: &str) -> Result<(), DatabaseError> {
        if !matches!(status, "upcoming" | "cleared" | "returned" | "cancelled") {
            return Err(DatabaseError::Validation("invalid status".into()));
        }
        self.connection.execute(
            "UPDATE fund_checks SET status=?1,updated_at=CURRENT_TIMESTAMP WHERE id=?2",
            params![status, id],
        )?;
        Ok(())
    }
}
