-- migrations/0008_inventory_history_visibility.sql

ALTER TABLE inventory_movements
ADD COLUMN visible_in_history INTEGER NOT NULL DEFAULT 1 CHECK (visible_in_history IN (0, 1));

CREATE INDEX idx_inventory_movements_visible_history
ON inventory_movements (visible_in_history, id DESC);
