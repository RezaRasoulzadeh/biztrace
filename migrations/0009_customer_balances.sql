-- migrations/0009_customer_balances.sql
--
-- Customers can carry a running balance with the business. A positive
-- balance means the customer owes money (debit / بدهکار); a negative
-- balance means the business owes the customer (credit / بستانکار).

ALTER TABLE customers ADD COLUMN balance_minor INTEGER NOT NULL DEFAULT 0;
ALTER TABLE customers ADD COLUMN currency TEXT NOT NULL DEFAULT 'IRR' CHECK (currency IN ('IRR', 'USD', 'EUR'));

CREATE INDEX idx_customers_balance ON customers(balance_minor);
