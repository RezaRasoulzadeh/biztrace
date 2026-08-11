-- migrations/0005_product_skus.sql

UPDATE catalog_items
SET sku = printf('NXR-P-%08d', id), updated_at = CURRENT_TIMESTAMP
WHERE kind = 'product' AND (sku IS NULL OR trim(sku) = '');

CREATE TRIGGER catalog_products_require_sku_insert
BEFORE INSERT ON catalog_items
WHEN NEW.kind = 'product' AND (NEW.sku IS NULL OR trim(NEW.sku) = '')
BEGIN
    SELECT RAISE(ABORT, 'product sku is required');
END;

CREATE TRIGGER catalog_products_require_sku_update
BEFORE UPDATE OF kind, sku ON catalog_items
WHEN NEW.kind = 'product' AND (NEW.sku IS NULL OR trim(NEW.sku) = '')
BEGIN
    SELECT RAISE(ABORT, 'product sku is required');
END;
