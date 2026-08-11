// tests/database.rs

use nexora::database::{CatalogDraft, Database};

#[test]
fn initial_schema_is_created_and_versioned() {
    let database = Database::open_in_memory().unwrap();
    assert_eq!(database.schema_version().unwrap(), 1);
    assert_eq!(database.overview_counts().unwrap(), Default::default());
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
