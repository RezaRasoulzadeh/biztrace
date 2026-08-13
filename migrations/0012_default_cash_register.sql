INSERT INTO fund_accounts (kind, name, account_number, opening_balance_minor, currency)
SELECT 'cash', 'صندوق اصلی', NULL, 0, 'IRR'
WHERE NOT EXISTS (SELECT 1 FROM fund_accounts WHERE active = 1 AND kind = 'cash');
