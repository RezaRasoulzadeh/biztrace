// src/main.rs

use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::rc::Rc;

use biztrace::database::{
    CatalogDraft, CatalogRecord, Database, DatabaseError, InventoryMovementDraft, MovementRecord,
    StockRecord, WarehouseRecord,
};
use biztrace::import::{
    read_catalog_excel, read_inventory_excel, write_catalog_export, write_catalog_template,
    write_inventory_export, write_inventory_template,
};
use rfd::AsyncFileDialog;
use slint::{ModelRc, SharedString, VecModel};

slint::include_modules!();

thread_local! {
    static CATALOG_SELECTION: RefCell<HashSet<i64>> = RefCell::new(HashSet::new());
    static INVENTORY_SELECTION: RefCell<HashSet<i64>> = RefCell::new(HashSet::new());
    static CATALOG_SORT: Cell<i32> = const { Cell::new(0) };
    static INVENTORY_SORT: Cell<i32> = const { Cell::new(0) };
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let database = Rc::new(Database::open_default()?);
    let counts = database.overview_counts()?;
    let app = AppWindow::new()?;
    app.set_invoice_count(counts.invoices);
    app.set_customer_count(counts.customers);
    app.set_catalog_count(counts.catalog_items);
    app.set_warehouse_count(counts.warehouses);
    app.set_fund_account_count(counts.fund_accounts);
    app.set_transaction_count(counts.fund_transactions);
    app.set_user_count(counts.users);
    app.set_catalog_items(catalog_model(database.catalog_items("")?));
    refresh_inventory(&app, &database, "")?;

    app.on_catalog_format_price(format_price_input);

    let weak = app.as_weak();
    let search_database = Rc::clone(&database);
    app.on_catalog_search(move |search| {
        if let (Some(app), Ok(records)) = (weak.upgrade(), search_database.catalog_items(&search)) {
            app.set_catalog_items(catalog_model(records));
        }
    });

    let weak = app.as_weak();
    let sort_database = Rc::clone(&database);
    app.on_catalog_sort_changed(move |index| {
        CATALOG_SORT.with(|sort| sort.set(index));
        if let Some(app) = weak.upgrade()
            && let Ok(records) = sort_database.catalog_items(&app.get_catalog_search_text())
        {
            app.set_catalog_items(catalog_model(records));
        }
    });

    let weak = app.as_weak();
    let selection_database = Rc::clone(&database);
    app.on_catalog_selection_changed(move |id, selected| {
        let Some(app) = weak.upgrade() else { return };
        if let Ok(id) = id.parse() {
            CATALOG_SELECTION.with(|selection| {
                if selected {
                    selection.borrow_mut().insert(id);
                } else {
                    selection.borrow_mut().remove(&id);
                }
                app.set_catalog_selected_count(selection.borrow().len() as i32);
            });
        }
        if let Ok(records) = selection_database.catalog_items(&app.get_catalog_search_text()) {
            app.set_catalog_items(catalog_model(records));
        }
    });

    let weak = app.as_weak();
    let select_all_database = Rc::clone(&database);
    app.on_catalog_select_all(move || {
        let Some(app) = weak.upgrade() else { return };
        if let Ok(records) = select_all_database.catalog_items(&app.get_catalog_search_text()) {
            CATALOG_SELECTION.with(|selection| {
                selection
                    .borrow_mut()
                    .extend(records.iter().map(|item| item.id));
                app.set_catalog_selected_count(selection.borrow().len() as i32);
            });
            app.set_catalog_items(catalog_model(records));
        }
    });

    let weak = app.as_weak();
    let clear_database = Rc::clone(&database);
    app.on_catalog_clear_selection(move || {
        let Some(app) = weak.upgrade() else { return };
        CATALOG_SELECTION.with(|selection| selection.borrow_mut().clear());
        app.set_catalog_selected_count(0);
        if let Ok(records) = clear_database.catalog_items(&app.get_catalog_search_text()) {
            app.set_catalog_items(catalog_model(records));
        }
    });

    let weak = app.as_weak();
    let export_database = Rc::clone(&database);
    app.on_catalog_export_all(move || {
        if let Some(app) = weak.upgrade() {
            export_catalog(&app, &export_database, false);
        }
    });
    let weak = app.as_weak();
    let export_database = Rc::clone(&database);
    app.on_catalog_export_selected(move || {
        if let Some(app) = weak.upgrade() {
            export_catalog(&app, &export_database, true);
        }
    });

    let weak = app.as_weak();
    let bulk_remove_database = Rc::clone(&database);
    app.on_catalog_remove_selected(move || {
        let Some(app) = weak.upgrade() else { return };
        let ids = CATALOG_SELECTION
            .with(|selection| selection.borrow().iter().copied().collect::<Vec<_>>());
        let mut removed = 0;
        let mut skipped = 0;
        for id in ids {
            if bulk_remove_database.remove_catalog_item(id).is_ok() {
                removed += 1;
            } else {
                skipped += 1;
            }
        }
        CATALOG_SELECTION.with(|selection| selection.borrow_mut().clear());
        app.set_catalog_selected_count(0);
        if let Ok(records) = bulk_remove_database.catalog_items(&app.get_catalog_search_text()) {
            app.set_catalog_items(catalog_model(records));
        }
        if let Ok(records) = bulk_remove_database.catalog_items("") {
            app.set_catalog_count(records.len() as i32);
        }
        app.set_status_message(
            format!("{removed} مورد حذف شد؛ {skipped} مورد استفاده‌شده نادیده گرفته شد").into(),
        );
        app.set_notification_open(true);
    });

    let weak = app.as_weak();
    let save_database = Rc::clone(&database);
    app.on_catalog_save(move |id, name, sku, kind_index, unit_index, price| {
        let Some(app) = weak.upgrade() else { return };
        app.set_catalog_editor_error("".into());
        let name = name.trim();
        if name.is_empty() {
            app.set_catalog_editor_error("نام کالا یا خدمت را وارد کنید".into());
            return;
        }
        let Some(price) = parse_amount(&price) else {
            app.set_catalog_editor_error("قیمت فروش را به‌صورت عدد معتبر وارد کنید".into());
            return;
        };
        let draft = CatalogDraft {
            id: if id.is_empty() { None } else { id.parse().ok() },
            kind: if kind_index == 0 {
                "product"
            } else {
                "service"
            }
            .into(),
            name: name.into(),
            sku: (!sku.trim().is_empty()).then(|| sku.trim().to_owned()),
            unit: unit_code(unit_index).into(),
            sale_price_minor: price,
        };
        match save_database.save_catalog_item(&draft) {
            Ok(_) => {
                let search = app.get_catalog_search_text();
                if let Ok(records) = save_database.catalog_items(&search) {
                    app.set_catalog_items(catalog_model(records));
                }
                if let Ok(records) = save_database.catalog_items("") {
                    app.set_catalog_count(records.len() as i32);
                }
                app.set_catalog_editor_open(false);
                app.set_status_message("کالا یا خدمت با موفقیت ذخیره شد".into());
                app.set_notification_open(true);
            }
            Err(_) => {
                app.set_catalog_editor_error("ذخیره انجام نشد؛ کد کالا نباید تکراری باشد".into());
            }
        }
    });

    let weak = app.as_weak();
    let remove_database = Rc::clone(&database);
    app.on_catalog_remove(move |id| {
        let Some(app) = weak.upgrade() else { return };
        let Some(id) = id.parse().ok() else { return };
        if remove_database.remove_catalog_item(id).is_ok() {
            CATALOG_SELECTION.with(|selection| {
                selection.borrow_mut().remove(&id);
                app.set_catalog_selected_count(selection.borrow().len() as i32);
            });
            let search = app.get_catalog_search_text();
            if let Ok(records) = remove_database.catalog_items(&search) {
                app.set_catalog_items(catalog_model(records));
            }
            if let Ok(records) = remove_database.catalog_items("") {
                app.set_catalog_count(records.len() as i32);
            }
            app.set_status_message("مورد انتخاب‌شده حذف شد".into());
            app.set_notification_open(true);
        } else {
            app.set_status_message(
                "این مورد در اسناد یا موجودی استفاده شده و قابل حذف نیست".into(),
            );
            app.set_notification_open(true);
        }
    });

    let weak = app.as_weak();
    app.on_catalog_download_template(move || {
        let weak = weak.clone();
        let _ = slint::spawn_local(async move {
            let Some(file) = AsyncFileDialog::new()
                .set_title("ذخیره فایل نمونه کالاها و خدمات")
                .set_file_name("biztrace-catalog-template.xlsx")
                .add_filter("Excel", &["xlsx"])
                .save_file()
                .await
            else {
                return;
            };
            let path = xlsx_path(file.path());
            let Some(app) = weak.upgrade() else { return };
            match safe_excel_write(|| write_catalog_template(&path)) {
                Ok(()) => app.set_status_message("فایل نمونه اکسل ذخیره شد".into()),
                Err(error) => {
                    app.set_status_message(format!("ذخیره فایل انجام نشد: {error}").into())
                }
            }
            app.set_notification_open(true);
        });
    });

    let weak = app.as_weak();
    let import_database = Rc::clone(&database);
    app.on_catalog_import_excel(move || {
        let weak = weak.clone();
        let import_database = Rc::clone(&import_database);
        let _ = slint::spawn_local(async move {
            let Some(file) = AsyncFileDialog::new()
                .set_title("انتخاب فایل کالاها و خدمات")
                .add_filter("Excel", &["xlsx", "xlsb", "xls", "ods"])
                .pick_file()
                .await
            else {
                return;
            };
            let result = read_catalog_excel(file.path()).and_then(|file| {
                import_database
                    .import_catalog_items(&file.rows)
                    .map(|result| (result, file.errors.len()))
                    .map_err(|error| error.to_string())
            });
            let Some(app) = weak.upgrade() else { return };
            match result {
                Ok((result, invalid)) => {
                    if let Ok(records) = import_database.catalog_items("") {
                        app.set_catalog_count(records.len() as i32);
                        app.set_catalog_items(catalog_model(records));
                    }
                    app.set_catalog_search_text("".into());
                    app.set_status_message(
                        format!(
                            "{} مورد وارد شد؛ {} تکراری و {} نامعتبر نادیده گرفته شد",
                            result.inserted, result.duplicates, invalid
                        )
                        .into(),
                    );
                }
                Err(error) => {
                    app.set_status_message(format!("ورود اکسل انجام نشد: {error}").into())
                }
            }
            app.set_notification_open(true);
        });
    });

    let weak = app.as_weak();
    let inventory_search_database = Rc::clone(&database);
    app.on_inventory_search(move |search| {
        if let (Some(app), Ok(records)) = (
            weak.upgrade(),
            inventory_search_database.stock_records(&search),
        ) && let Ok(warehouses) = inventory_search_database.warehouses()
        {
            app.set_inventory_items(stock_model(records, &warehouses));
        }
    });

    let weak = app.as_weak();
    let inventory_sort_database = Rc::clone(&database);
    app.on_inventory_sort_changed(move |index| {
        INVENTORY_SORT.with(|sort| sort.set(index));
        if let Some(app) = weak.upgrade()
            && let (Ok(records), Ok(warehouses)) = (
                inventory_sort_database.stock_records(&app.get_inventory_search_text()),
                inventory_sort_database.warehouses(),
            )
        {
            app.set_inventory_items(stock_model(records, &warehouses));
        }
    });

    let weak = app.as_weak();
    let inventory_selection_database = Rc::clone(&database);
    app.on_inventory_selection_changed(move |id, selected| {
        let Some(app) = weak.upgrade() else { return };
        if let Ok(id) = id.parse() {
            INVENTORY_SELECTION.with(|selection| {
                if selected {
                    selection.borrow_mut().insert(id);
                } else {
                    selection.borrow_mut().remove(&id);
                }
                app.set_inventory_selected_count(selection.borrow().len() as i32);
            });
        }
        if let (Ok(records), Ok(warehouses)) = (
            inventory_selection_database.stock_records(&app.get_inventory_search_text()),
            inventory_selection_database.warehouses(),
        ) {
            app.set_inventory_items(stock_model(records, &warehouses));
        }
    });

    let weak = app.as_weak();
    let inventory_select_all_database = Rc::clone(&database);
    app.on_inventory_select_all(move || {
        let Some(app) = weak.upgrade() else { return };
        if let (Ok(records), Ok(warehouses)) = (
            inventory_select_all_database.stock_records(&app.get_inventory_search_text()),
            inventory_select_all_database.warehouses(),
        ) {
            INVENTORY_SELECTION.with(|selection| {
                selection
                    .borrow_mut()
                    .extend(records.iter().map(|item| item.cost_layer_id));
                app.set_inventory_selected_count(selection.borrow().len() as i32);
            });
            app.set_inventory_items(stock_model(records, &warehouses));
        }
    });

    let weak = app.as_weak();
    let inventory_clear_database = Rc::clone(&database);
    app.on_inventory_clear_selection(move || {
        let Some(app) = weak.upgrade() else { return };
        INVENTORY_SELECTION.with(|selection| selection.borrow_mut().clear());
        app.set_inventory_selected_count(0);
        if let (Ok(records), Ok(warehouses)) = (
            inventory_clear_database.stock_records(&app.get_inventory_search_text()),
            inventory_clear_database.warehouses(),
        ) {
            app.set_inventory_items(stock_model(records, &warehouses));
        }
    });

    let weak = app.as_weak();
    let inventory_export_database = Rc::clone(&database);
    app.on_inventory_export_all(move || {
        if let Some(app) = weak.upgrade() {
            export_inventory(&app, &inventory_export_database, false);
        }
    });
    let weak = app.as_weak();
    let inventory_export_database = Rc::clone(&database);
    app.on_inventory_export_selected(move || {
        if let Some(app) = weak.upgrade() {
            export_inventory(&app, &inventory_export_database, true);
        }
    });

    let weak = app.as_weak();
    let inventory_bulk_remove_database = Rc::clone(&database);
    app.on_inventory_remove_selected(move || {
        let Some(app) = weak.upgrade() else { return };
        let ids = INVENTORY_SELECTION
            .with(|selection| selection.borrow().iter().copied().collect::<Vec<_>>());
        let mut removed = 0;
        let mut skipped = 0;
        for id in ids {
            if inventory_bulk_remove_database.remove_cost_layer(id).is_ok() {
                removed += 1;
            } else {
                skipped += 1;
            }
        }
        INVENTORY_SELECTION.with(|selection| selection.borrow_mut().clear());
        app.set_inventory_selected_count(0);
        let search = app.get_inventory_search_text();
        let _ = refresh_inventory(&app, &inventory_bulk_remove_database, &search);
        app.set_status_message(
            format!("{removed} ردیف موجودی حذف شد؛ {skipped} ردیف نادیده گرفته شد").into(),
        );
        app.set_notification_open(true);
    });

    let weak = app.as_weak();
    let warehouse_database = Rc::clone(&database);
    app.on_save_warehouse(move |id, name, address| {
        let Some(app) = weak.upgrade() else { return };
        app.set_inventory_editor_error("".into());
        if name.trim().is_empty() {
            app.set_inventory_editor_error("نام انبار را وارد کنید".into());
            return;
        }
        let address = (!address.trim().is_empty()).then(|| address.trim().to_owned());
        let id = if id.is_empty() { None } else { id.parse().ok() };
        match warehouse_database.save_warehouse(id, name.trim(), address.as_deref()) {
            Ok(_) => {
                if refresh_inventory(&app, &warehouse_database, "").is_ok() {
                    app.set_warehouse_editor_open(false);
                    app.set_status_message("انبار ذخیره شد".into());
                    app.set_notification_open(true);
                }
            }
            Err(_) => app.set_inventory_editor_error("نام انبار نباید تکراری باشد".into()),
        }
    });

    let weak = app.as_weak();
    let remove_warehouse_database = Rc::clone(&database);
    app.on_remove_warehouse(move |id| {
        let Some(app) = weak.upgrade() else { return };
        app.set_inventory_editor_error("".into());
        let Some(id) = id.parse().ok() else { return };
        match remove_warehouse_database.remove_warehouse(id) {
            Ok(()) => {
                if refresh_inventory(&app, &remove_warehouse_database, "").is_ok() {
                    app.set_warehouse_removal_open(false);
                    app.set_status_message("انبار حذف شد و سوابق آن حفظ شدند".into());
                    app.set_notification_open(true);
                }
            }
            Err(DatabaseError::Validation(_)) => app.set_inventory_editor_error(
                "این انبار موجودی دارد؛ آن را دستی مدیریت کنید یا انتقال اجباری را انتخاب کنید"
                    .into(),
            ),
            Err(_) => app.set_inventory_editor_error("حذف انبار انجام نشد".into()),
        }
    });

    let weak = app.as_weak();
    let warehouse_choices_database = Rc::clone(&database);
    app.on_prepare_warehouse_removal(move |source_id| {
        let Some(app) = weak.upgrade() else { return };
        let Some(source_id) = source_id.parse::<i64>().ok() else {
            return;
        };
        if let Ok(warehouses) = warehouse_choices_database.warehouses() {
            let alternatives: Vec<_> = warehouses
                .into_iter()
                .filter(|warehouse| warehouse.id != source_id)
                .collect();
            app.set_inventory_warehouse_alternative_names(string_model(
                alternatives.iter().map(|item| item.name.clone()).collect(),
            ));
            app.set_inventory_warehouse_alternative_ids(string_model(
                alternatives
                    .into_iter()
                    .map(|item| item.id.to_string())
                    .collect(),
            ));
            app.set_inventory_editor_error("".into());
        }
    });

    let weak = app.as_weak();
    let force_remove_database = Rc::clone(&database);
    app.on_force_remove_warehouse(move |source_id, target_id| {
        let Some(app) = weak.upgrade() else { return };
        app.set_inventory_editor_error("".into());
        let Some(source_id) = source_id.parse().ok() else {
            return;
        };
        let Some(target_id) = target_id.parse().ok() else {
            app.set_inventory_editor_error("انبار جایگزین را انتخاب کنید".into());
            return;
        };
        let target_name = force_remove_database
            .warehouses()
            .ok()
            .and_then(|warehouses| {
                warehouses
                    .into_iter()
                    .find(|warehouse| warehouse.id == target_id)
            })
            .map(|warehouse| warehouse.name)
            .unwrap_or_else(|| "انبار جایگزین".into());
        match force_remove_database.move_and_remove_warehouse(source_id, target_id) {
            Ok(()) => {
                if refresh_inventory(&app, &force_remove_database, "").is_ok() {
                    app.set_warehouse_removal_open(false);
                    app.set_status_message(
                        format!("همه موجودی‌ها به «{target_name}» منتقل و انبار حذف شد").into(),
                    );
                    app.set_notification_open(true);
                }
            }
            Err(_) => app.set_inventory_editor_error("انتقال موجودی و حذف انبار انجام نشد".into()),
        }
    });

    let weak = app.as_weak();
    let product_search_database = Rc::clone(&database);
    app.on_inventory_product_search(move |search| {
        if let (Some(app), Ok(records)) = (
            weak.upgrade(),
            product_search_database.inventory_products(&search),
        ) {
            app.set_inventory_product_options(product_option_model(records));
        }
    });

    let weak = app.as_weak();
    let movement_database = Rc::clone(&database);
    app.on_record_inventory_movement(
        move |warehouse_index,
              product_id,
              layer_id,
              direction_index,
              quantity,
              unit_cost,
              reference| {
            let Some(app) = weak.upgrade() else { return };
            app.set_inventory_editor_error("".into());
            let Ok(warehouses) = movement_database.warehouses() else {
                return;
            };
            let Some(warehouse) = warehouses.get(warehouse_index as usize) else {
                app.set_inventory_editor_error("ابتدا یک انبار ایجاد کنید".into());
                return;
            };
            let Some(product_id) = product_id.parse().ok() else {
                app.set_inventory_editor_error("ابتدا یک کالا در بخش کالاها ثبت کنید".into());
                return;
            };
            let Some(quantity) = parse_quantity(&quantity) else {
                app.set_inventory_editor_error("مقدار واردشده معتبر نیست".into());
                return;
            };
            let cost_layer_id = if layer_id.is_empty() {
                None
            } else {
                let Some(layer_id) = layer_id.parse().ok() else {
                    app.set_inventory_editor_error("ردیف موجودی معتبر نیست".into());
                    return;
                };
                Some(layer_id)
            };
            let unit_cost_minor = if direction_index == 0 {
                let Some(value) = parse_amount(&unit_cost) else {
                    app.set_inventory_editor_error("قیمت خرید هر واحد را وارد کنید".into());
                    return;
                };
                Some(value)
            } else {
                None
            };
            let draft = InventoryMovementDraft {
                warehouse_id: warehouse.id,
                item_id: product_id,
                cost_layer_id,
                quantity_milliunits: quantity,
                increases_stock: direction_index == 0,
                unit_cost_minor,
                reference: (!reference.trim().is_empty()).then(|| reference.trim().to_owned()),
            };
            match movement_database.record_inventory_movement(&draft) {
                Ok(()) => {
                    let search = app.get_inventory_search_text();
                    if refresh_inventory(&app, &movement_database, &search).is_ok() {
                        app.set_movement_editor_open(false);
                        app.set_status_message("تغییر موجودی ثبت شد".into());
                        app.set_notification_open(true);
                    }
                }
                Err(DatabaseError::Validation(_)) => {
                    app.set_inventory_editor_error("موجودی برای این خروج کافی نیست".into());
                }
                Err(_) => app.set_inventory_editor_error("ثبت موجودی انجام نشد".into()),
            }
        },
    );

    let weak = app.as_weak();
    let stock_database = Rc::clone(&database);
    app.on_set_stock_level(
        move |layer_id, warehouse_index, quantity, unit_cost, reference| {
            let Some(app) = weak.upgrade() else { return };
            app.set_inventory_editor_error("".into());
            let (Some(layer_id), Some(quantity)) =
                (layer_id.parse().ok(), parse_nonnegative_quantity(&quantity))
            else {
                app.set_inventory_editor_error("موجودی نهایی را به‌صورت عدد معتبر وارد کنید".into());
                return;
            };
            let reference = (!reference.trim().is_empty()).then(|| reference.trim().to_owned());
            let Some(unit_cost_minor) = parse_amount(&unit_cost) else {
                app.set_inventory_editor_error("بهای خرید هر واحد را وارد کنید".into());
                return;
            };
            let Ok(warehouses) = stock_database.warehouses() else {
                app.set_inventory_editor_error("خواندن فهرست انبارها انجام نشد".into());
                return;
            };
            let Some(warehouse) = warehouses.get(warehouse_index as usize) else {
                app.set_inventory_editor_error("انبار مقصد را انتخاب کنید".into());
                return;
            };
            match stock_database.update_cost_layer(
                layer_id,
                warehouse.id,
                quantity,
                unit_cost_minor,
                reference.as_deref(),
            ) {
                Ok(()) => {
                    let search = app.get_inventory_search_text();
                    if refresh_inventory(&app, &stock_database, &search).is_ok() {
                        app.set_stock_editor_open(false);
                        app.set_status_message("موجودی نهایی اصلاح شد".into());
                        app.set_notification_open(true);
                    }
                }
                Err(_) => app.set_inventory_editor_error("اصلاح موجودی انجام نشد".into()),
            }
        },
    );

    let weak = app.as_weak();
    let remove_stock_database = Rc::clone(&database);
    app.on_remove_stock_layer(move |layer_id| {
        let Some(app) = weak.upgrade() else { return };
        let Some(layer_id) = layer_id.parse().ok() else {
            return;
        };
        match remove_stock_database.remove_cost_layer(layer_id) {
            Ok(()) => {
                INVENTORY_SELECTION.with(|selection| {
                    selection.borrow_mut().remove(&layer_id);
                    app.set_inventory_selected_count(selection.borrow().len() as i32);
                });
                let search = app.get_inventory_search_text();
                if refresh_inventory(&app, &remove_stock_database, &search).is_ok() {
                    app.set_status_message("ردیف موجودی حذف شد".into());
                    app.set_notification_open(true);
                }
            }
            Err(_) => {
                app.set_status_message("حذف ردیف موجودی انجام نشد".into());
                app.set_notification_open(true);
            }
        }
    });

    let weak = app.as_weak();
    app.on_inventory_download_template(move || {
        let weak = weak.clone();
        let _ = slint::spawn_local(async move {
            let Some(file) = AsyncFileDialog::new()
                .set_title("ذخیره فایل نمونه موجودی")
                .set_file_name("biztrace-inventory-template.xlsx")
                .add_filter("Excel", &["xlsx"])
                .save_file()
                .await
            else {
                return;
            };
            let path = xlsx_path(file.path());
            let Some(app) = weak.upgrade() else { return };
            match safe_excel_write(|| write_inventory_template(&path)) {
                Ok(()) => app.set_status_message("فایل نمونه موجودی ذخیره شد".into()),
                Err(error) => {
                    app.set_status_message(format!("ذخیره فایل انجام نشد: {error}").into())
                }
            }
            app.set_notification_open(true);
        });
    });

    let weak = app.as_weak();
    let inventory_import_database = Rc::clone(&database);
    app.on_inventory_import_excel(move || {
        let weak = weak.clone();
        let inventory_import_database = Rc::clone(&inventory_import_database);
        let _ = slint::spawn_local(async move {
            let Some(file) = AsyncFileDialog::new()
                .set_title("انتخاب فایل ورود موجودی")
                .add_filter("Excel", &["xlsx", "xlsb", "xls", "ods"])
                .pick_file()
                .await
            else {
                return;
            };
            let result = read_inventory_excel(file.path());
            let Some(app) = weak.upgrade() else { return };
            match result {
                Ok(file) => match inventory_import_database.import_inventory_rows(&file.rows) {
                    Ok(result) => {
                        let search = app.get_inventory_search_text();
                        let _ = refresh_inventory(&app, &inventory_import_database, &search);
                        app.set_status_message(
                            format!(
                                "{} ردیف وارد شد؛ {} ردیف نادیده گرفته شد",
                                result.inserted,
                                result.skipped + file.errors.len()
                            )
                            .into(),
                        );
                    }
                    Err(error) => {
                        app.set_status_message(format!("ورود موجودی انجام نشد: {error}").into())
                    }
                },
                Err(error) => {
                    app.set_status_message(format!("خواندن فایل انجام نشد: {error}").into())
                }
            }
            app.set_notification_open(true);
        });
    });

    Ok(app.run()?)
}

fn export_catalog(app: &AppWindow, database: &Database, selected_only: bool) {
    let Ok(mut records) = database.catalog_items("") else {
        return;
    };
    if selected_only {
        CATALOG_SELECTION
            .with(|selection| records.retain(|item| selection.borrow().contains(&item.id)));
        if records.is_empty() {
            app.set_status_message("موردی برای خروجی انتخاب نشده است".into());
            app.set_notification_open(true);
            return;
        }
    }
    let weak = app.as_weak();
    let _ = slint::spawn_local(async move {
        let Some(file) = AsyncFileDialog::new()
            .set_title("ذخیره خروجی کالاها")
            .set_file_name("biztrace-catalog-export.xlsx")
            .add_filter("Excel", &["xlsx"])
            .save_file()
            .await
        else {
            return;
        };
        let path = xlsx_path(file.path());
        let Some(app) = weak.upgrade() else { return };
        match safe_excel_write(|| write_catalog_export(&path, &records)) {
            Ok(()) => {
                app.set_status_message(format!("خروجی {} مورد ذخیره شد", records.len()).into())
            }
            Err(error) => app.set_status_message(format!("خروجی ذخیره نشد: {error}").into()),
        }
        app.set_notification_open(true);
    });
}

fn export_inventory(app: &AppWindow, database: &Database, selected_only: bool) {
    let Ok(mut records) = database.stock_records("") else {
        return;
    };
    if selected_only {
        INVENTORY_SELECTION.with(|selection| {
            records.retain(|item| selection.borrow().contains(&item.cost_layer_id))
        });
        if records.is_empty() {
            app.set_status_message("ردیفی برای خروجی انتخاب نشده است".into());
            app.set_notification_open(true);
            return;
        }
    }
    let weak = app.as_weak();
    let _ = slint::spawn_local(async move {
        let Some(file) = AsyncFileDialog::new()
            .set_title("ذخیره خروجی موجودی")
            .set_file_name("biztrace-inventory-export.xlsx")
            .add_filter("Excel", &["xlsx"])
            .save_file()
            .await
        else {
            return;
        };
        let path = xlsx_path(file.path());
        let Some(app) = weak.upgrade() else { return };
        match safe_excel_write(|| write_inventory_export(&path, &records)) {
            Ok(()) => {
                app.set_status_message(format!("خروجی {} ردیف ذخیره شد", records.len()).into())
            }
            Err(error) => app.set_status_message(format!("خروجی ذخیره نشد: {error}").into()),
        }
        app.set_notification_open(true);
    });
}

fn xlsx_path(path: &std::path::Path) -> std::path::PathBuf {
    let mut path = path.to_path_buf();
    if !path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("xlsx"))
    {
        path.set_extension("xlsx");
    }
    path
}

fn safe_excel_write(operation: impl FnOnce() -> Result<(), String>) -> Result<(), String> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(operation))
        .map_err(|_| "ساخت فایل اکسل با خطای داخلی متوقف شد".to_owned())?
}

fn catalog_model(mut records: Vec<CatalogRecord>) -> ModelRc<CatalogItemData> {
    CATALOG_SORT.with(|sort| match sort.get() {
        1 => records.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase())),
        2 => records.sort_by(|a, b| b.name.to_lowercase().cmp(&a.name.to_lowercase())),
        3 => records.sort_by_key(|item| item.sale_price_minor),
        4 => records.sort_by_key(|item| std::cmp::Reverse(item.sale_price_minor)),
        _ => {}
    });
    let rows: Vec<CatalogItemData> = records
        .into_iter()
        .map(|record| CatalogItemData {
            id: record.id.to_string().into(),
            name: record.name.into(),
            sku: record.sku.into(),
            kind_code: record.kind.clone().into(),
            kind_label: if record.kind == "product" {
                "کالا"
            } else {
                "خدمت"
            }
            .into(),
            unit_code: record.unit.clone().into(),
            unit_label: unit_label(&record.unit).into(),
            price_label: format_amount(record.sale_price_minor),
            price_value: format_number(record.sale_price_minor).into(),
            selected: CATALOG_SELECTION.with(|selected| selected.borrow().contains(&record.id)),
        })
        .collect();
    ModelRc::new(VecModel::from(rows))
}

fn parse_amount(value: &str) -> Option<i64> {
    let normalized: String = value
        .chars()
        .filter_map(|character| match character {
            '0'..='9' => Some(character),
            '۰'..='۹' => char::from_digit(character as u32 - '۰' as u32, 10),
            '٠'..='٩' => char::from_digit(character as u32 - '٠' as u32, 10),
            ',' | '٬' | ' ' => None,
            _ => Some('\0'),
        })
        .collect();
    if normalized.is_empty() || normalized.contains('\0') {
        return None;
    }
    normalized.parse().ok()
}

fn format_amount(value: i64) -> SharedString {
    format!("{} ریال", format_number(value)).into()
}

fn format_price_input(value: SharedString, cursor: i32) -> FormattedInput {
    let cursor = usize::try_from(cursor)
        .unwrap_or(value.len())
        .min(value.len());
    let digits_to_right = value[cursor..]
        .chars()
        .filter(|character| {
            character.is_ascii_digit() || matches!(character, '۰'..='۹' | '٠'..='٩')
        })
        .count();
    let digits: String = value
        .chars()
        .filter_map(|character| match character {
            '0'..='9' => Some(character),
            '۰'..='۹' => char::from_digit(character as u32 - '۰' as u32, 10),
            '٠'..='٩' => char::from_digit(character as u32 - '٠' as u32, 10),
            _ => None,
        })
        .collect();
    if digits.is_empty() {
        return FormattedInput {
            text: "".into(),
            cursor: 0,
        };
    }
    let formatted = digits.parse::<i64>().map(format_number).unwrap_or(digits);
    let cursor = cursor_for_digits_to_right(&formatted, digits_to_right);
    FormattedInput {
        text: formatted.into(),
        cursor: cursor as i32,
    }
}

fn cursor_for_digits_to_right(value: &str, digits_to_right: usize) -> usize {
    if digits_to_right == 0 {
        return value.len();
    }
    let mut remaining = value
        .chars()
        .filter(|character| character.is_ascii_digit())
        .count();
    for (offset, character) in value.char_indices() {
        if remaining == digits_to_right {
            return offset;
        }
        if character.is_ascii_digit() {
            remaining -= 1;
        }
    }
    value.len()
}

fn format_number(value: i64) -> String {
    let digits = value.to_string();
    let mut output = String::new();
    for (index, character) in digits.chars().rev().enumerate() {
        if index > 0 && index % 3 == 0 {
            output.push(',');
        }
        output.push(character);
    }
    output.chars().rev().collect()
}

fn unit_code(index: i32) -> &'static str {
    match index {
        0 => "each",
        1 => "kilogram",
        2 => "gram",
        3 => "liter",
        4 => "meter",
        5 => "hour",
        6 => "session",
        _ => "custom",
    }
}

fn unit_label(code: &str) -> &'static str {
    match code {
        "each" => "عدد",
        "kilogram" => "کیلوگرم",
        "gram" => "گرم",
        "liter" => "لیتر",
        "meter" => "متر",
        "hour" => "ساعت",
        "session" => "جلسه",
        _ => "سفارشی",
    }
}

fn refresh_inventory(
    app: &AppWindow,
    database: &Database,
    search: &str,
) -> Result<(), DatabaseError> {
    let warehouses = database.warehouses()?;
    let records = database.stock_records(search)?;
    app.set_inventory_items(stock_model(records, &warehouses));
    app.set_warehouse_count(warehouses.len() as i32);
    app.set_inventory_warehouse_items(warehouse_model(warehouses.clone()));
    app.set_inventory_warehouses(string_model(
        warehouses.iter().map(|item| item.name.clone()).collect(),
    ));
    app.set_inventory_product_options(product_option_model(database.inventory_products("")?));
    app.set_inventory_movements(movement_model(database.movement_records()?));
    Ok(())
}

fn stock_model(
    mut records: Vec<StockRecord>,
    warehouses: &[WarehouseRecord],
) -> ModelRc<StockItemData> {
    INVENTORY_SORT.with(|sort| match sort.get() {
        1 => records.sort_by(|a, b| a.item_name.to_lowercase().cmp(&b.item_name.to_lowercase())),
        2 => records.sort_by_key(|item| item.quantity_milliunits),
        3 => records.sort_by_key(|item| std::cmp::Reverse(item.quantity_milliunits)),
        4 => records.sort_by_key(|item| item.unit_cost_minor),
        5 => records.sort_by_key(|item| std::cmp::Reverse(item.unit_cost_minor)),
        _ => records.sort_by_key(|item| std::cmp::Reverse(item.cost_layer_id)),
    });
    let rows: Vec<StockItemData> = records
        .into_iter()
        .map(|record| StockItemData {
            cost_layer_id: record.cost_layer_id.to_string().into(),
            warehouse_id: record.warehouse_id.to_string().into(),
            item_id: record.item_id.to_string().into(),
            item_name: record.item_name.into(),
            sku: record.sku.into(),
            warehouse_name: record.warehouse_name.into(),
            warehouse_index: warehouses
                .iter()
                .position(|warehouse| warehouse.id == record.warehouse_id)
                .unwrap_or_default() as i32,
            unit_label: unit_label(&record.unit).into(),
            quantity_label: format_quantity(record.quantity_milliunits).into(),
            received_quantity_label: format_quantity(record.acquired_quantity_milliunits).into(),
            remaining_progress: if record.acquired_quantity_milliunits == 0 {
                0.0
            } else {
                (record.quantity_milliunits as f32 / record.acquired_quantity_milliunits as f32)
                    .clamp(0.0, 1.0)
            },
            quantity_value: format_quantity(record.quantity_milliunits).into(),
            inventory_value_label: format_amount(record.inventory_value_minor),
            unit_cost_label: format_amount(record.unit_cost_minor),
            unit_cost_value: format_number(record.unit_cost_minor).into(),
            selected: INVENTORY_SELECTION
                .with(|selected| selected.borrow().contains(&record.cost_layer_id)),
        })
        .collect();
    ModelRc::new(VecModel::from(rows))
}

fn movement_model(records: Vec<MovementRecord>) -> ModelRc<MovementItemData> {
    ModelRc::new(VecModel::from(
        records
            .into_iter()
            .map(|record| MovementItemData {
                item_name: record.item_name.into(),
                warehouse_name: record.warehouse_name.into(),
                quantity_label: format_quantity(record.quantity_milliunits).into(),
                increases_stock: record.increases_stock,
                reference: record.reference.into(),
                occurred_on: record.occurred_on.into(),
                unit_cost_label: record
                    .unit_cost_minor
                    .map(format_amount)
                    .unwrap_or_else(|| "—".into()),
                total_cost_label: record
                    .total_cost_minor
                    .map(format_amount)
                    .unwrap_or_else(|| "—".into()),
            })
            .collect::<Vec<_>>(),
    ))
}

fn warehouse_model(records: Vec<WarehouseRecord>) -> ModelRc<WarehouseItemData> {
    ModelRc::new(VecModel::from(
        records
            .into_iter()
            .map(|record| WarehouseItemData {
                id: record.id.to_string().into(),
                name: record.name.into(),
                address: record.address.into(),
                has_stock: record.has_stock,
            })
            .collect::<Vec<_>>(),
    ))
}

fn product_option_model(records: Vec<(i64, String, String)>) -> ModelRc<SearchOption> {
    ModelRc::new(VecModel::from(
        records
            .into_iter()
            .map(|(id, name, sku)| SearchOption {
                id: id.to_string().into(),
                label: name.into(),
                detail: if sku.is_empty() {
                    "بدون کد"
                } else {
                    &sku
                }
                .into(),
            })
            .collect::<Vec<_>>(),
    ))
}

fn string_model(values: Vec<String>) -> ModelRc<SharedString> {
    ModelRc::new(VecModel::from(
        values
            .into_iter()
            .map(SharedString::from)
            .collect::<Vec<_>>(),
    ))
}

fn parse_quantity(value: &str) -> Option<i64> {
    parse_quantity_value(value).filter(|value| *value > 0)
}

fn parse_nonnegative_quantity(value: &str) -> Option<i64> {
    parse_quantity_value(value)
}

fn parse_quantity_value(value: &str) -> Option<i64> {
    let normalized: String = value
        .chars()
        .filter_map(|character| match character {
            '0'..='9' | '.' => Some(character),
            '۰'..='۹' => char::from_digit(character as u32 - '۰' as u32, 10),
            '٠'..='٩' => char::from_digit(character as u32 - '٠' as u32, 10),
            '٫' | '/' => Some('.'),
            ',' | '٬' | ' ' => None,
            _ => Some('\0'),
        })
        .collect();
    if normalized.is_empty() || normalized.contains('\0') {
        return None;
    }
    let (whole, fraction) = normalized.split_once('.').unwrap_or((&normalized, ""));
    if fraction.len() > 3 || fraction.contains('.') {
        return None;
    }
    let whole = if whole.is_empty() {
        0
    } else {
        whole.parse::<i64>().ok()?
    };
    let fraction = format!("{fraction:0<3}").parse::<i64>().ok()?;
    whole.checked_mul(1_000)?.checked_add(fraction)
}

fn format_quantity(value: i64) -> String {
    let whole = value / 1_000;
    let fraction = value % 1_000;
    if fraction == 0 {
        whole.to_string()
    } else {
        format!("{whole}.{fraction:03}")
            .trim_end_matches('0')
            .to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn price_formatter_groups_digits_and_keeps_end_caret() {
        let result = format_price_input("1000".into(), 4);
        assert_eq!(result.text, "1,000");
        assert_eq!(result.cursor, 5);
    }

    #[test]
    fn price_formatter_preserves_digits_to_the_right_of_caret() {
        let result = format_price_input("12000".into(), 2);
        assert_eq!(result.text, "12,000");
        assert_eq!(result.cursor, 2);
    }

    #[test]
    fn quantity_parser_supports_fractional_persian_input() {
        assert_eq!(parse_quantity("۱۲٫۵"), Some(12_500));
        assert_eq!(parse_quantity("۱/۵"), Some(1_500));
        assert_eq!(parse_quantity(".125"), Some(125));
        assert_eq!(format_quantity(12_500), "12.5");
    }
}
