-- migrations/0002_optional_inventory_actor.sql

CREATE TABLE inventory_movements_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    warehouse_id INTEGER NOT NULL REFERENCES warehouses(id) ON DELETE RESTRICT,
    item_id INTEGER NOT NULL REFERENCES catalog_items(id) ON DELETE RESTRICT,
    kind TEXT NOT NULL CHECK (kind IN ('opening', 'purchase', 'sale', 'customer_return', 'supplier_return', 'adjustment', 'transfer_in', 'transfer_out')),
    quantity_milliunits INTEGER NOT NULL CHECK (quantity_milliunits > 0),
    increases_stock INTEGER NOT NULL CHECK (increases_stock IN (0, 1)),
    occurred_on TEXT NOT NULL,
    reference TEXT,
    note TEXT,
    created_by INTEGER REFERENCES users(id) ON DELETE RESTRICT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO inventory_movements_new
SELECT id, warehouse_id, item_id, kind, quantity_milliunits, increases_stock, occurred_on, reference, note, created_by, created_at
FROM inventory_movements;

DROP TABLE inventory_movements;
ALTER TABLE inventory_movements_new RENAME TO inventory_movements;

CREATE INDEX idx_inventory_movements_item_date ON inventory_movements(item_id, occurred_on);
CREATE INDEX idx_inventory_movements_warehouse_date ON inventory_movements(warehouse_id, occurred_on);
