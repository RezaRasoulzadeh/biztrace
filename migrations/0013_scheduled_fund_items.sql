ALTER TABLE fund_checks ADD COLUMN schedule_type TEXT NOT NULL DEFAULT 'check'
    CHECK (schedule_type IN ('check', 'installment', 'scheduled'));

CREATE INDEX IF NOT EXISTS idx_fund_checks_schedule_type
    ON fund_checks(schedule_type, direction, status, due_on);
