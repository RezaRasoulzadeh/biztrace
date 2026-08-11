-- migrations/0006_inventory_revision_warehouses.sql

ALTER TABLE inventory_cost_layer_revisions
ADD COLUMN previous_warehouse_id INTEGER REFERENCES warehouses(id) ON DELETE RESTRICT;

ALTER TABLE inventory_cost_layer_revisions
ADD COLUMN new_warehouse_id INTEGER REFERENCES warehouses(id) ON DELETE RESTRICT;
