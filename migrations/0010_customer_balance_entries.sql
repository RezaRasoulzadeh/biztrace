CREATE TABLE customer_balance_entries (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    customer_id INTEGER NOT NULL REFERENCES customers(id) ON DELETE RESTRICT,
    amount_minor INTEGER NOT NULL CHECK (amount_minor <> 0),
    kind TEXT NOT NULL CHECK (kind IN ('debit', 'credit', 'settlement')),
    note TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_customer_balance_entries_customer_date
    ON customer_balance_entries(customer_id, created_at);
