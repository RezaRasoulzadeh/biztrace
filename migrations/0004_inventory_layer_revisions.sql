-- migrations/0004_inventory_layer_revisions.sql

CREATE TABLE inventory_cost_layer_revisions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    cost_layer_id INTEGER NOT NULL REFERENCES inventory_cost_layers(id) ON DELETE RESTRICT,
    previous_quantity_milliunits INTEGER NOT NULL CHECK (previous_quantity_milliunits >= 0),
    new_quantity_milliunits INTEGER NOT NULL CHECK (new_quantity_milliunits >= 0),
    previous_unit_cost_minor INTEGER NOT NULL CHECK (previous_unit_cost_minor >= 0),
    new_unit_cost_minor INTEGER NOT NULL CHECK (new_unit_cost_minor >= 0),
    reference TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_inventory_cost_layer_revisions_layer
ON inventory_cost_layer_revisions (cost_layer_id, id);
