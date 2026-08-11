-- migrations/0003_inventory_cost_layers.sql

ALTER TABLE inventory_movements ADD COLUMN unit_cost_minor INTEGER CHECK (unit_cost_minor >= 0);
ALTER TABLE inventory_movements ADD COLUMN total_cost_minor INTEGER CHECK (total_cost_minor >= 0);

CREATE TABLE inventory_cost_layers (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    warehouse_id INTEGER NOT NULL REFERENCES warehouses(id) ON DELETE RESTRICT,
    item_id INTEGER NOT NULL REFERENCES catalog_items(id) ON DELETE RESTRICT,
    source_movement_id INTEGER REFERENCES inventory_movements(id) ON DELETE RESTRICT,
    acquired_quantity_milliunits INTEGER NOT NULL CHECK (acquired_quantity_milliunits > 0),
    remaining_quantity_milliunits INTEGER NOT NULL CHECK (remaining_quantity_milliunits >= 0),
    unit_cost_minor INTEGER NOT NULL CHECK (unit_cost_minor >= 0),
    acquired_on TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE inventory_cost_allocations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    movement_id INTEGER NOT NULL REFERENCES inventory_movements(id) ON DELETE CASCADE,
    cost_layer_id INTEGER NOT NULL REFERENCES inventory_cost_layers(id) ON DELETE RESTRICT,
    quantity_milliunits INTEGER NOT NULL CHECK (quantity_milliunits > 0),
    unit_cost_minor INTEGER NOT NULL CHECK (unit_cost_minor >= 0),
    allocated_cost_minor INTEGER NOT NULL CHECK (allocated_cost_minor >= 0)
);

INSERT INTO inventory_cost_layers
(warehouse_id, item_id, acquired_quantity_milliunits, remaining_quantity_milliunits, unit_cost_minor, acquired_on)
SELECT warehouse_id, item_id, quantity_milliunits, quantity_milliunits, 0, date('now')
FROM stock_levels
WHERE quantity_milliunits > 0;

CREATE INDEX idx_inventory_cost_layers_fifo
ON inventory_cost_layers (warehouse_id, item_id, acquired_on, id);

CREATE INDEX idx_inventory_cost_allocations_movement
ON inventory_cost_allocations (movement_id);
