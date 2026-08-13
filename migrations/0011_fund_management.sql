ALTER TABLE fund_accounts ADD COLUMN kind TEXT NOT NULL DEFAULT 'bank' CHECK (kind IN ('cash', 'bank', 'card', 'other'));
ALTER TABLE fund_accounts ADD COLUMN account_number TEXT;
ALTER TABLE fund_accounts ADD COLUMN updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP;
DROP INDEX IF EXISTS idx_fund_transactions_account_date;
DROP INDEX IF EXISTS idx_fund_transactions_invoice;
ALTER TABLE fund_transactions RENAME TO fund_transactions_old;
CREATE TABLE fund_transactions (
 id INTEGER PRIMARY KEY AUTOINCREMENT, account_id INTEGER NOT NULL REFERENCES fund_accounts(id) ON DELETE RESTRICT,
 transfer_account_id INTEGER REFERENCES fund_accounts(id) ON DELETE RESTRICT, kind TEXT NOT NULL CHECK(kind IN ('income','expense','transfer')),
 amount_minor INTEGER NOT NULL CHECK(amount_minor>0), currency TEXT NOT NULL DEFAULT 'IRR', category TEXT NOT NULL, occurred_on TEXT NOT NULL,
 reference TEXT, invoice_id INTEGER REFERENCES invoices(id) ON DELETE SET NULL, description TEXT, created_by INTEGER REFERENCES users(id) ON DELETE RESTRICT,
 created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP, updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
 CHECK((kind='transfer' AND transfer_account_id IS NOT NULL AND transfer_account_id<>account_id) OR (kind<>'transfer' AND transfer_account_id IS NULL))
);
INSERT INTO fund_transactions(id,account_id,transfer_account_id,kind,amount_minor,currency,category,occurred_on,invoice_id,description,created_by,created_at)
SELECT id,account_id,transfer_account_id,kind,amount_minor,currency,category,occurred_on,invoice_id,description,created_by,created_at FROM fund_transactions_old;
DROP TABLE fund_transactions_old;
CREATE INDEX idx_fund_transactions_account_date ON fund_transactions(account_id,occurred_on);
CREATE INDEX idx_fund_transactions_invoice ON fund_transactions(invoice_id);
CREATE UNIQUE INDEX idx_fund_transactions_reference ON fund_transactions(reference) WHERE reference IS NOT NULL;

CREATE TABLE fund_checks (
 id INTEGER PRIMARY KEY AUTOINCREMENT,
 direction TEXT NOT NULL CHECK(direction IN ('incoming','outgoing')),
 account_id INTEGER NOT NULL REFERENCES fund_accounts(id) ON DELETE RESTRICT,
 party_name TEXT NOT NULL,
 check_number TEXT NOT NULL,
 bank_name TEXT,
 amount_minor INTEGER NOT NULL CHECK(amount_minor>0),
 currency TEXT NOT NULL DEFAULT 'IRR',
 due_on TEXT NOT NULL,
 status TEXT NOT NULL DEFAULT 'upcoming' CHECK(status IN ('upcoming','cleared','returned','cancelled')),
 note TEXT,
 created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
 updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
 UNIQUE(direction,check_number)
);
CREATE INDEX idx_fund_checks_due_status ON fund_checks(status,due_on);
