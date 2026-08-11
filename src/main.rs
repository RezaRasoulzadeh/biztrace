// src/main.rs

use nexora::database::Database;

slint::include_modules!();

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let database = Database::open_default()?;
    let counts = database.overview_counts()?;
    let app = AppWindow::new()?;
    app.set_invoice_count(counts.invoices);
    app.set_customer_count(counts.customers);
    app.set_catalog_count(counts.catalog_items);
    app.set_warehouse_count(counts.warehouses);
    app.set_fund_account_count(counts.fund_accounts);
    app.set_transaction_count(counts.fund_transactions);
    app.set_user_count(counts.users);

    Ok(app.run()?)
}
