// src/main.rs

use std::rc::Rc;

use nexora::database::{CatalogDraft, CatalogRecord, Database};
use nexora::import::{read_catalog_excel, write_catalog_template};
use rfd::FileDialog;
use slint::{ModelRc, SharedString, VecModel};

slint::include_modules!();

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

    app.on_catalog_format_price(format_price_input);

    let weak = app.as_weak();
    let search_database = Rc::clone(&database);
    app.on_catalog_search(move |search| {
        if let (Some(app), Ok(records)) = (weak.upgrade(), search_database.catalog_items(&search)) {
            app.set_catalog_items(catalog_model(records));
        }
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
        let Some(app) = weak.upgrade() else { return };
        let Some(mut path) = FileDialog::new()
            .set_title("ذخیره فایل نمونه کالاها و خدمات")
            .set_file_name("nexora-catalog-template.xlsx")
            .add_filter("Excel", &["xlsx"])
            .save_file()
        else {
            return;
        };
        if path.extension().is_none() {
            path.set_extension("xlsx");
        }
        match write_catalog_template(&path) {
            Ok(()) => app.set_status_message("فایل نمونه اکسل ذخیره شد".into()),
            Err(error) => app.set_status_message(format!("ذخیره فایل انجام نشد: {error}").into()),
        }
        app.set_notification_open(true);
    });

    let weak = app.as_weak();
    let import_database = Rc::clone(&database);
    app.on_catalog_import_excel(move || {
        let Some(app) = weak.upgrade() else { return };
        let Some(path) = FileDialog::new()
            .set_title("انتخاب فایل کالاها و خدمات")
            .add_filter("Excel", &["xlsx", "xlsb", "xls", "ods"])
            .pick_file()
        else {
            return;
        };
        let result = read_catalog_excel(&path).and_then(|drafts| {
            import_database
                .import_catalog_items(&drafts)
                .map_err(|error| error.to_string())
        });
        match result {
            Ok(result) => {
                if let Ok(records) = import_database.catalog_items("") {
                    app.set_catalog_count(records.len() as i32);
                    app.set_catalog_items(catalog_model(records));
                }
                app.set_catalog_search_text("".into());
                app.set_status_message(
                    format!(
                        "{} مورد وارد شد؛ {} مورد تکراری نادیده گرفته شد",
                        result.inserted, result.duplicates
                    )
                    .into(),
                );
            }
            Err(error) => app.set_status_message(format!("ورود اکسل انجام نشد: {error}").into()),
        }
        app.set_notification_open(true);
    });

    Ok(app.run()?)
}

fn catalog_model(records: Vec<CatalogRecord>) -> ModelRc<CatalogItemData> {
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
}
