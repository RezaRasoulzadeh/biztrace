// tests/models.rs

use nexora::models::{
    CatalogItem, CatalogItemId, Currency, Date, Invoice, InvoiceId, InvoiceLine, InvoiceStatus,
    ItemKind, ModelError, Money, Quantity, Unit, UserId,
};

#[test]
fn products_track_inventory_and_services_do_not() {
    let price = Money::new(1_000, Currency::Irr);
    let product = CatalogItem::new(
        CatalogItemId(1),
        ItemKind::Product,
        "Tea",
        Unit::Each,
        price,
    )
    .unwrap();
    let service = CatalogItem::new(
        CatalogItemId(2),
        ItemKind::Service,
        "Repair",
        Unit::Hour,
        price,
    )
    .unwrap();

    assert!(product.track_inventory);
    assert!(!service.track_inventory);
}

#[test]
fn invoice_total_uses_fixed_point_quantity_discount_and_tax() {
    let line = InvoiceLine {
        item_id: Some(CatalogItemId(1)),
        item_kind: ItemKind::Product,
        description: "Tea".into(),
        quantity: Quantity::from_milliunits(2_000).unwrap(),
        unit_price: Money::new(100_000, Currency::Irr),
        discount: Money::new(10_000, Currency::Irr),
        tax_basis_points: 1_000,
    };
    let invoice = Invoice {
        id: InvoiceId(1),
        number: "INV-1".into(),
        customer_id: None,
        status: InvoiceStatus::Draft,
        issued_on: Date::new(2026, 8, 12).unwrap(),
        due_on: None,
        currency: Currency::Irr,
        lines: vec![line],
        notes: None,
        created_by: UserId(1),
    };

    assert_eq!(invoice.total().unwrap(), Money::new(209_000, Currency::Irr));
}

#[test]
fn money_rejects_mixed_currencies() {
    let result = Money::new(1, Currency::Irr) + Money::new(1, Currency::Usd);
    assert_eq!(result, Err(ModelError::CurrencyMismatch));
}

#[test]
fn date_validation_rejects_impossible_dates() {
    assert_eq!(Date::new(2025, 2, 29), Err(ModelError::InvalidDate));
    assert!(Date::new(2024, 2, 29).is_ok());
}
