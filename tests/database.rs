// tests/database.rs

use biztrace::database::{
    CatalogDraft, CustomerDraft, Database, InventoryImportRow, InventoryMovementDraft,
};

#[test]
fn initial_schema_is_created_and_versioned() {
    let database = Database::open_in_memory().unwrap();
    assert_eq!(database.schema_version().unwrap(), 10);
    assert_eq!(database.overview_counts().unwrap(), Default::default());
}

#[test]
fn customer_balances_preserve_debit_credit_and_settled_values() {
    let database = Database::open_in_memory().unwrap();
    for name in ["Debit", "Credit", "Settled"] {
        let id = database
            .save_customer(&CustomerDraft {
                id: None,
                kind: "individual".into(),
                name: name.into(),
                phone: None,
                email: None,
                address: None,
            })
            .unwrap();
        let adjustment = match name {
            "Debit" => 250_000,
            "Credit" => -125_000,
            _ => 0,
        };
        if adjustment != 0 {
            database
                .adjust_customer_balance(id, adjustment, Some("test"))
                .unwrap();
        }
    }
    let customers = database.customers("").unwrap();
    assert_eq!(
        customers
            .iter()
            .find(|item| item.name == "Debit")
            .unwrap()
            .balance_minor,
        250_000
    );
    assert_eq!(
        customers
            .iter()
            .find(|item| item.name == "Credit")
            .unwrap()
            .balance_minor,
        -125_000
    );
    assert_eq!(
        customers
            .iter()
            .find(|item| item.name == "Settled")
            .unwrap()
            .balance_minor,
        0
    );
    let debit = customers.iter().find(|item| item.name == "Debit").unwrap();
    database
        .save_customer(&CustomerDraft {
            id: Some(debit.id),
            kind: debit.kind.clone(),
            name: "Debit edited".into(),
            phone: Some("09120000000".into()),
            email: None,
            address: None,
        })
        .unwrap();
    assert_eq!(
        database.customer(debit.id).unwrap().unwrap().balance_minor,
        250_000
    );
    let credit_id = customers
        .iter()
        .find(|item| item.name == "Credit")
        .unwrap()
        .id;
    database.settle_customer_balance(credit_id).unwrap();
    assert_eq!(
        database.customer(credit_id).unwrap().unwrap().balance_minor,
        0
    );
    let settled_id = customers
        .iter()
        .find(|item| item.name == "Settled")
        .unwrap()
        .id;
    database.remove_customer(settled_id).unwrap();
    assert_eq!(database.overview_counts().unwrap().customers, 2);
}

#[test]
fn fifo_cost_layers_preserve_purchase_prices_for_profit_calculation() {
    let database = Database::open_in_memory().unwrap();
    let item_id = database
        .save_catalog_item(&CatalogDraft {
            id: None,
            kind: "product".into(),
            name: "Coffee".into(),
            sku: Some("COFFEE-FIFO".into()),
            unit: "kilogram".into(),
            sale_price_minor: 70_000_000,
        })
        .unwrap();
    let warehouse_id = database.create_warehouse("Main", None).unwrap();

    for (quantity_milliunits, unit_cost_minor) in [(100_000, 40_000_000), (100_000, 50_000_000)] {
        database
            .record_inventory_movement(&InventoryMovementDraft {
                warehouse_id,
                item_id,
                cost_layer_id: None,
                quantity_milliunits,
                increases_stock: true,
                unit_cost_minor: Some(unit_cost_minor),
                reference: None,
            })
            .unwrap();
    }
    database
        .record_inventory_movement(&InventoryMovementDraft {
            warehouse_id,
            item_id,
            cost_layer_id: None,
            quantity_milliunits: 150_000,
            increases_stock: false,
            unit_cost_minor: None,
            reference: None,
        })
        .unwrap();

    let issue = database.movement_records().unwrap().remove(0);
    assert_eq!(issue.total_cost_minor, Some(6_500_000_000));
    assert_eq!(issue.unit_cost_minor, Some(43_333_333));
    let remaining = database.stock_records("").unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].acquired_quantity_milliunits, 100_000);
    assert_eq!(remaining[0].quantity_milliunits, 50_000);
}

#[test]
fn row_outgoing_decreases_only_the_selected_cost_layer() {
    let database = Database::open_in_memory().unwrap();
    let item_id = database
        .save_catalog_item(&CatalogDraft {
            id: None,
            kind: "product".into(),
            name: "Layered coffee".into(),
            sku: Some("COFFEE-LAYERS".into()),
            unit: "kilogram".into(),
            sale_price_minor: 70_000_000,
        })
        .unwrap();
    let warehouse_id = database.create_warehouse("Main", None).unwrap();
    for unit_cost_minor in [40_000_000, 50_000_000] {
        database
            .record_inventory_movement(&InventoryMovementDraft {
                warehouse_id,
                item_id,
                cost_layer_id: None,
                quantity_milliunits: 100_000,
                increases_stock: true,
                unit_cost_minor: Some(unit_cost_minor),
                reference: None,
            })
            .unwrap();
    }
    let layers = database.stock_records("").unwrap();
    let selected_layer_id = layers
        .iter()
        .find(|layer| layer.unit_cost_minor == 50_000_000)
        .unwrap()
        .cost_layer_id;
    database
        .record_inventory_movement(&InventoryMovementDraft {
            warehouse_id,
            item_id,
            cost_layer_id: Some(selected_layer_id),
            quantity_milliunits: 25_000,
            increases_stock: false,
            unit_cost_minor: None,
            reference: None,
        })
        .unwrap();

    let layers = database.stock_records("").unwrap();
    assert_eq!(
        layers
            .iter()
            .find(|layer| layer.unit_cost_minor == 40_000_000)
            .unwrap()
            .quantity_milliunits,
        100_000
    );
    assert_eq!(
        layers
            .iter()
            .find(|layer| layer.unit_cost_minor == 50_000_000)
            .unwrap()
            .quantity_milliunits,
        75_000
    );
}

#[test]
fn inventory_batch_quantity_and_unit_cost_can_be_edited() {
    let database = Database::open_in_memory().unwrap();
    let item_id = database
        .save_catalog_item(&CatalogDraft {
            id: None,
            kind: "product".into(),
            name: "Coffee batch".into(),
            sku: Some("COFFEE-EDIT".into()),
            unit: "kilogram".into(),
            sale_price_minor: 70_000_000,
        })
        .unwrap();
    let warehouse_id = database.create_warehouse("Batch warehouse", None).unwrap();
    database
        .record_inventory_movement(&InventoryMovementDraft {
            warehouse_id,
            item_id,
            cost_layer_id: None,
            quantity_milliunits: 100_000,
            increases_stock: true,
            unit_cost_minor: Some(40_000_000),
            reference: None,
        })
        .unwrap();

    let layer_id = database.stock_records("").unwrap()[0].cost_layer_id;
    database
        .update_cost_layer(
            layer_id,
            warehouse_id,
            90_000,
            42_000_000,
            Some("invoice correction"),
        )
        .unwrap();

    let record = &database.stock_records("").unwrap()[0];
    assert_eq!(record.quantity_milliunits, 90_000);
    assert_eq!(record.acquired_quantity_milliunits, 90_000);
    assert_eq!(record.unit_cost_minor, 42_000_000);
    assert_eq!(record.inventory_value_minor, 3_780_000_000);

    let target_warehouse = database.create_warehouse("Target warehouse", None).unwrap();
    database
        .update_cost_layer(
            layer_id,
            target_warehouse,
            90_000,
            42_000_000,
            Some("warehouse correction"),
        )
        .unwrap();
    assert_eq!(
        database.stock_records("").unwrap()[0].warehouse_id,
        target_warehouse
    );

    database.remove_cost_layer(layer_id).unwrap();
    assert!(database.stock_records("").unwrap().is_empty());
}

#[test]
fn inventory_excel_rows_import_valid_entries_and_skip_unknown_skus() {
    let database = Database::open_in_memory().unwrap();
    database
        .save_catalog_item(&CatalogDraft {
            id: None,
            kind: "product".into(),
            name: "Arabica".into(),
            sku: Some("COF-ARABICA".into()),
            unit: "kilogram".into(),
            sale_price_minor: 60_000_000,
        })
        .unwrap();
    database.create_warehouse("Main", None).unwrap();
    let rows = [
        InventoryImportRow {
            warehouse: "Main".into(),
            sku: "COF-ARABICA".into(),
            quantity_milliunits: 100_000,
            unit_cost_minor: 40_000_000,
            reference: None,
        },
        InventoryImportRow {
            warehouse: "Main".into(),
            sku: "UNKNOWN".into(),
            quantity_milliunits: 10_000,
            unit_cost_minor: 1_000,
            reference: None,
        },
    ];

    let result = database.import_inventory_rows(&rows).unwrap();
    assert_eq!(result.inserted, 1);
    assert_eq!(result.skipped, 1);
    assert_eq!(database.stock_records("").unwrap().len(), 1);
}

#[test]
fn populated_warehouse_requires_transfer_before_removal() {
    let database = Database::open_in_memory().unwrap();
    let item_id = database
        .save_catalog_item(&CatalogDraft {
            id: None,
            kind: "product".into(),
            name: "Transfer coffee".into(),
            sku: None,
            unit: "kilogram".into(),
            sale_price_minor: 60_000_000,
        })
        .unwrap();
    let source = database.create_warehouse("Source", None).unwrap();
    let target = database.create_warehouse("Target", None).unwrap();
    database
        .record_inventory_movement(&InventoryMovementDraft {
            warehouse_id: source,
            item_id,
            cost_layer_id: None,
            quantity_milliunits: 25_000,
            increases_stock: true,
            unit_cost_minor: Some(40_000_000),
            reference: None,
        })
        .unwrap();

    assert!(database.remove_warehouse(source).is_err());
    database.move_and_remove_warehouse(source, target).unwrap();

    let warehouses = database.warehouses().unwrap();
    assert_eq!(warehouses.len(), 1);
    assert_eq!(warehouses[0].id, target);
    let stock = database.stock_records("").unwrap();
    assert_eq!(stock[0].warehouse_id, target);
    assert_eq!(stock[0].quantity_milliunits, 25_000);
}

#[test]
fn migrations_are_idempotent_for_new_connections() {
    let first = Database::open_in_memory().unwrap();
    let second = Database::open_in_memory().unwrap();
    assert_eq!(
        first.schema_version().unwrap(),
        second.schema_version().unwrap()
    );
}

#[test]
fn catalog_items_can_be_created_searched_updated_and_removed() {
    let database = Database::open_in_memory().unwrap();
    let id = database
        .save_catalog_item(&CatalogDraft {
            id: None,
            kind: "product".into(),
            name: "چای احمد".into(),
            sku: Some("TEA-1".into()),
            unit: "each".into(),
            sale_price_minor: 1_250_000,
        })
        .unwrap();

    assert_eq!(database.catalog_items("چای").unwrap().len(), 1);
    database
        .save_catalog_item(&CatalogDraft {
            id: Some(id),
            kind: "product".into(),
            name: "چای احمد ۵۰۰ گرمی".into(),
            sku: Some("TEA-1".into()),
            unit: "each".into(),
            sale_price_minor: 1_300_000,
        })
        .unwrap();
    assert_eq!(
        database.catalog_item(id).unwrap().unwrap().sale_price_minor,
        1_300_000
    );

    database.remove_catalog_item(id).unwrap();
    assert!(database.catalog_items("").unwrap().is_empty());
}

#[test]
fn archived_inventory_keeps_history_without_blocking_catalog_removal() {
    let database = Database::open_in_memory().unwrap();
    let item_id = database
        .save_catalog_item(&CatalogDraft {
            id: None,
            kind: "product".into(),
            name: "Archived coffee".into(),
            sku: Some("COFFEE-ARCHIVED".into()),
            unit: "kilogram".into(),
            sale_price_minor: 70_000_000,
        })
        .unwrap();
    let warehouse_id = database.create_warehouse("Main", None).unwrap();
    database
        .record_inventory_movement(&InventoryMovementDraft {
            warehouse_id,
            item_id,
            cost_layer_id: None,
            quantity_milliunits: 10_000,
            increases_stock: true,
            unit_cost_minor: Some(40_000_000),
            reference: Some("purchase log".into()),
        })
        .unwrap();

    assert!(database.remove_catalog_item(item_id).is_err());
    let layer_id = database.stock_records("").unwrap()[0].cost_layer_id;
    database.remove_cost_layer(layer_id).unwrap();
    database.remove_catalog_item(item_id).unwrap();

    assert!(database.catalog_item(item_id).unwrap().is_none());
    assert!(database.stock_records("").unwrap().is_empty());
    assert!(database.movement_records().unwrap().is_empty());
}

#[test]
fn history_only_shows_movements_for_inventory_rows_that_still_exist() {
    let database = Database::open_in_memory().unwrap();
    let item_id = database
        .save_catalog_item(&CatalogDraft {
            id: None,
            kind: "product".into(),
            name: "Two batches".into(),
            sku: Some("TWO-BATCHES".into()),
            unit: "each".into(),
            sale_price_minor: 1_000,
        })
        .unwrap();
    let warehouse_id = database.create_warehouse("Main", None).unwrap();
    for reference in ["first batch", "second batch"] {
        database
            .record_inventory_movement(&InventoryMovementDraft {
                warehouse_id,
                item_id,
                cost_layer_id: None,
                quantity_milliunits: 10_000,
                increases_stock: true,
                unit_cost_minor: Some(500),
                reference: Some(reference.into()),
            })
            .unwrap();
    }
    let first_layer = database
        .stock_records("")
        .unwrap()
        .into_iter()
        .min_by_key(|layer| layer.cost_layer_id)
        .unwrap();
    database
        .remove_cost_layer(first_layer.cost_layer_id)
        .unwrap();

    let history = database.movement_records().unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].reference, "second batch");
}

#[test]
fn products_receive_a_unique_generated_sku_when_omitted() {
    let database = Database::open_in_memory().unwrap();
    let first = database
        .save_catalog_item(&CatalogDraft {
            id: None,
            kind: "product".into(),
            name: "Coffee one".into(),
            sku: None,
            unit: "kilogram".into(),
            sale_price_minor: 1_000,
        })
        .unwrap();
    let second = database
        .save_catalog_item(&CatalogDraft {
            id: None,
            kind: "product".into(),
            name: "Coffee two".into(),
            sku: None,
            unit: "kilogram".into(),
            sale_price_minor: 2_000,
        })
        .unwrap();

    let first_sku = database.catalog_item(first).unwrap().unwrap().sku;
    let second_sku = database.catalog_item(second).unwrap().unwrap().sku;
    assert!(first_sku.starts_with("NXR-P-"));
    assert!(second_sku.starts_with("NXR-P-"));
    assert_ne!(first_sku, second_sku);
}

#[test]
fn catalog_bulk_import_inserts_all_rows() {
    let database = Database::open_in_memory().unwrap();
    let drafts = vec![
        CatalogDraft {
            id: None,
            kind: "product".into(),
            name: "Product".into(),
            sku: Some("BULK-1".into()),
            unit: "each".into(),
            sale_price_minor: 10_000,
        },
        CatalogDraft {
            id: None,
            kind: "service".into(),
            name: "Service".into(),
            sku: None,
            unit: "hour".into(),
            sale_price_minor: 20_000,
        },
    ];

    let result = database.import_catalog_items(&drafts).unwrap();
    assert_eq!(result.inserted, 2);
    assert_eq!(result.duplicates, 0);
    assert_eq!(database.catalog_items("").unwrap().len(), 2);

    let result = database.import_catalog_items(&drafts).unwrap();
    assert_eq!(result.inserted, 0);
    assert_eq!(result.duplicates, 2);
}

#[test]
fn inventory_movements_update_stock_and_reject_negative_balance() {
    let database = Database::open_in_memory().unwrap();
    let item_id = database
        .save_catalog_item(&CatalogDraft {
            id: None,
            kind: "product".into(),
            name: "Coffee".into(),
            sku: Some("COFFEE-1".into()),
            unit: "kilogram".into(),
            sale_price_minor: 1_000,
        })
        .unwrap();
    let warehouse_id = database.create_warehouse("Main", None).unwrap();

    database
        .record_inventory_movement(&InventoryMovementDraft {
            warehouse_id,
            item_id,
            cost_layer_id: None,
            quantity_milliunits: 5_000,
            increases_stock: true,
            unit_cost_minor: Some(40_000_000),
            reference: None,
        })
        .unwrap();
    database
        .record_inventory_movement(&InventoryMovementDraft {
            warehouse_id,
            item_id,
            cost_layer_id: None,
            quantity_milliunits: 2_000,
            increases_stock: false,
            unit_cost_minor: None,
            reference: None,
        })
        .unwrap();

    assert_eq!(
        database.stock_records("").unwrap()[0].quantity_milliunits,
        3_000
    );
    assert!(
        database
            .record_inventory_movement(&InventoryMovementDraft {
                warehouse_id,
                item_id,
                cost_layer_id: None,
                quantity_milliunits: 4_000,
                increases_stock: false,
                unit_cost_minor: None,
                reference: None,
            })
            .is_err()
    );
}

#[test]
fn fractional_outgoing_stock_preserves_exact_milliunits() {
    let database = Database::open_in_memory().unwrap();
    let item_id = database
        .save_catalog_item(&CatalogDraft {
            id: None,
            kind: "product".into(),
            name: "Ground coffee".into(),
            sku: Some("COFFEE-FRACTIONAL".into()),
            unit: "kilogram".into(),
            sale_price_minor: 60_000_000,
        })
        .unwrap();
    let warehouse_id = database.create_warehouse("Main", None).unwrap();
    database
        .record_inventory_movement(&InventoryMovementDraft {
            warehouse_id,
            item_id,
            cost_layer_id: None,
            quantity_milliunits: 1_500,
            increases_stock: true,
            unit_cost_minor: Some(40_000_000),
            reference: None,
        })
        .unwrap();
    database
        .record_inventory_movement(&InventoryMovementDraft {
            warehouse_id,
            item_id,
            cost_layer_id: None,
            quantity_milliunits: 125,
            increases_stock: false,
            unit_cost_minor: None,
            reference: None,
        })
        .unwrap();

    let stock = database.stock_records("").unwrap();
    assert_eq!(stock[0].quantity_milliunits, 1_375);
    assert_eq!(stock[0].inventory_value_minor, 55_000_000);
}
