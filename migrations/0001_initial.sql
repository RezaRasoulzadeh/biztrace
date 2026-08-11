-- migrations/0001_initial.sql

CREATE TABLE users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    username TEXT NOT NULL COLLATE NOCASE UNIQUE,
    display_name TEXT NOT NULL,
    password_hash TEXT NOT NULL,
    role TEXT NOT NULL CHECK (role IN ('owner', 'administrator', 'manager', 'sales', 'accountant', 'inventory', 'staff')),
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'suspended')),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE customers (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    kind TEXT NOT NULL CHECK (kind IN ('individual', 'business')),
    name TEXT NOT NULL,
    phone TEXT,
    email TEXT,
    address TEXT,
    tax_id TEXT,
    notes TEXT,
    active INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1)),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE catalog_items (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    kind TEXT NOT NULL CHECK (kind IN ('product', 'service')),
    name TEXT NOT NULL,
    sku TEXT COLLATE NOCASE UNIQUE,
    description TEXT,
    unit TEXT NOT NULL CHECK (unit IN ('each', 'kilogram', 'gram', 'liter', 'meter', 'hour', 'session', 'custom')),
    sale_price_minor INTEGER NOT NULL CHECK (sale_price_minor >= 0),
    cost_price_minor INTEGER CHECK (cost_price_minor >= 0),
    currency TEXT NOT NULL CHECK (currency IN ('IRR', 'USD', 'EUR')),
    tax_basis_points INTEGER NOT NULL DEFAULT 0 CHECK (tax_basis_points BETWEEN 0 AND 10000),
    track_inventory INTEGER NOT NULL CHECK (track_inventory IN (0, 1)),
    active INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1)),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CHECK (kind = 'product' OR track_inventory = 0)
);

CREATE TABLE warehouses (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    address TEXT,
    active INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1)),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE stock_levels (
    warehouse_id INTEGER NOT NULL REFERENCES warehouses(id) ON DELETE RESTRICT,
    item_id INTEGER NOT NULL REFERENCES catalog_items(id) ON DELETE RESTRICT,
    quantity_milliunits INTEGER NOT NULL DEFAULT 0,
    reorder_point_milliunits INTEGER CHECK (reorder_point_milliunits >= 0),
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (warehouse_id, item_id)
);

CREATE TABLE inventory_movements (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    warehouse_id INTEGER NOT NULL REFERENCES warehouses(id) ON DELETE RESTRICT,
    item_id INTEGER NOT NULL REFERENCES catalog_items(id) ON DELETE RESTRICT,
    kind TEXT NOT NULL CHECK (kind IN ('opening', 'purchase', 'sale', 'customer_return', 'supplier_return', 'adjustment', 'transfer_in', 'transfer_out')),
    quantity_milliunits INTEGER NOT NULL CHECK (quantity_milliunits > 0),
    increases_stock INTEGER NOT NULL CHECK (increases_stock IN (0, 1)),
    occurred_on TEXT NOT NULL,
    reference TEXT,
    note TEXT,
    created_by INTEGER NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE invoices (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    number TEXT NOT NULL COLLATE NOCASE UNIQUE,
    customer_id INTEGER REFERENCES customers(id) ON DELETE RESTRICT,
    status TEXT NOT NULL DEFAULT 'draft' CHECK (status IN ('draft', 'issued', 'partially_paid', 'paid', 'voided')),
    issued_on TEXT NOT NULL,
    due_on TEXT,
    currency TEXT NOT NULL CHECK (currency IN ('IRR', 'USD', 'EUR')),
    notes TEXT,
    created_by INTEGER NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CHECK (due_on IS NULL OR due_on >= issued_on)
);

CREATE TABLE invoice_lines (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    invoice_id INTEGER NOT NULL REFERENCES invoices(id) ON DELETE CASCADE,
    position INTEGER NOT NULL CHECK (position >= 0),
    item_id INTEGER REFERENCES catalog_items(id) ON DELETE RESTRICT,
    item_kind TEXT NOT NULL CHECK (item_kind IN ('product', 'service')),
    description TEXT NOT NULL,
    quantity_milliunits INTEGER NOT NULL CHECK (quantity_milliunits > 0),
    unit_price_minor INTEGER NOT NULL CHECK (unit_price_minor >= 0),
    discount_minor INTEGER NOT NULL DEFAULT 0 CHECK (discount_minor >= 0),
    tax_basis_points INTEGER NOT NULL DEFAULT 0 CHECK (tax_basis_points BETWEEN 0 AND 10000),
    UNIQUE (invoice_id, position)
);

CREATE TABLE fund_accounts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    currency TEXT NOT NULL CHECK (currency IN ('IRR', 'USD', 'EUR')),
    opening_balance_minor INTEGER NOT NULL DEFAULT 0,
    active INTEGER NOT NULL DEFAULT 1 CHECK (active IN (0, 1)),
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (name, currency)
);

CREATE TABLE fund_transactions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id INTEGER NOT NULL REFERENCES fund_accounts(id) ON DELETE RESTRICT,
    transfer_account_id INTEGER REFERENCES fund_accounts(id) ON DELETE RESTRICT,
    kind TEXT NOT NULL CHECK (kind IN ('income', 'expense', 'transfer')),
    amount_minor INTEGER NOT NULL CHECK (amount_minor > 0),
    currency TEXT NOT NULL CHECK (currency IN ('IRR', 'USD', 'EUR')),
    category TEXT NOT NULL,
    occurred_on TEXT NOT NULL,
    invoice_id INTEGER REFERENCES invoices(id) ON DELETE SET NULL,
    description TEXT,
    created_by INTEGER NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CHECK ((kind = 'transfer' AND transfer_account_id IS NOT NULL AND transfer_account_id <> account_id) OR (kind <> 'transfer' AND transfer_account_id IS NULL))
);

CREATE INDEX idx_customers_name ON customers(name);
CREATE INDEX idx_customers_phone ON customers(phone);
CREATE INDEX idx_catalog_items_name ON catalog_items(name);
CREATE INDEX idx_catalog_items_kind_active ON catalog_items(kind, active);
CREATE INDEX idx_stock_levels_item ON stock_levels(item_id);
CREATE INDEX idx_inventory_movements_item_date ON inventory_movements(item_id, occurred_on);
CREATE INDEX idx_inventory_movements_warehouse_date ON inventory_movements(warehouse_id, occurred_on);
CREATE INDEX idx_invoices_customer_date ON invoices(customer_id, issued_on);
CREATE INDEX idx_invoices_status_date ON invoices(status, issued_on);
CREATE INDEX idx_invoice_lines_item ON invoice_lines(item_id);
CREATE INDEX idx_fund_transactions_account_date ON fund_transactions(account_id, occurred_on);
CREATE INDEX idx_fund_transactions_invoice ON fund_transactions(invoice_id);
