-- migrations/0007_inventory_archiving.sql

ALTER TABLE inventory_cost_layers ADD COLUMN archived_at TEXT;

CREATE INDEX idx_inventory_cost_layers_active
ON inventory_cost_layers (warehouse_id, item_id, archived_at, remaining_quantity_milliunits);
