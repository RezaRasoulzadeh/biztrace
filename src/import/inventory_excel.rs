// src/import/inventory_excel.rs

use std::path::Path;

use calamine::{Data, Reader, open_workbook_auto};
use rust_xlsxwriter::Workbook;

use crate::database::{InventoryImportRow, StockRecord};

pub struct InventoryImportFile {
    pub rows: Vec<InventoryImportRow>,
    pub errors: Vec<String>,
}

const HEADERS: [&str; 5] = ["انبار", "کد کالا", "مقدار", "بهای خرید واحد (ریال)", "مرجع"];

pub fn write_inventory_template(path: &Path) -> Result<(), String> {
    let mut workbook = Workbook::new();
    let sheet = workbook.add_worksheet();
    sheet
        .set_name("ورود موجودی")
        .map_err(|error| error.to_string())?;
    sheet.set_right_to_left(true);
    for (column, header) in HEADERS.iter().enumerate() {
        sheet
            .write_string(0, column as u16, *header)
            .map_err(|error| error.to_string())?;
    }
    let samples = [
        [
            "انبار مرکزی",
            "COF-ARABICA-1KG",
            "100",
            "40000000",
            "خرید مرداد",
        ],
        [
            "انبار مرکزی",
            "DRK-ESPRESSO",
            "50",
            "1200000",
            "فاکتور ۱۰۰۲",
        ],
    ];
    for (row, values) in samples.iter().enumerate() {
        for (column, value) in values.iter().enumerate() {
            sheet
                .write_string((row + 1) as u32, column as u16, *value)
                .map_err(|error| error.to_string())?;
        }
    }
    for (column, width) in [20.0, 24.0, 14.0, 24.0, 24.0].iter().enumerate() {
        sheet
            .set_column_width(column as u16, *width)
            .map_err(|error| error.to_string())?;
    }
    workbook.save(path).map_err(|error| error.to_string())
}

pub fn write_inventory_export(path: &Path, records: &[StockRecord]) -> Result<(), String> {
    let mut workbook = Workbook::new();
    let sheet = workbook.add_worksheet();
    sheet
        .set_name("ورود موجودی")
        .map_err(|error| error.to_string())?;
    sheet.set_right_to_left(true);
    for (column, header) in HEADERS.iter().enumerate() {
        sheet
            .write_string(0, column as u16, *header)
            .map_err(|error| error.to_string())?;
    }
    for (index, record) in records.iter().enumerate() {
        let row = (index + 1) as u32;
        sheet
            .write_string(row, 0, &record.warehouse_name)
            .map_err(|error| error.to_string())?;
        sheet
            .write_string(row, 1, &record.sku)
            .map_err(|error| error.to_string())?;
        sheet
            .write_number(row, 2, record.quantity_milliunits as f64 / 1_000.0)
            .map_err(|error| error.to_string())?;
        sheet
            .write_number(row, 3, record.unit_cost_minor as f64)
            .map_err(|error| error.to_string())?;
        sheet
            .write_string(row, 4, "خروجی BizTrace")
            .map_err(|error| error.to_string())?;
    }
    for (column, width) in [20.0, 24.0, 14.0, 24.0, 24.0].iter().enumerate() {
        sheet
            .set_column_width(column as u16, *width)
            .map_err(|error| error.to_string())?;
    }
    workbook.save(path).map_err(|error| error.to_string())
}

pub fn read_inventory_excel(path: &Path) -> Result<InventoryImportFile, String> {
    let mut workbook = open_workbook_auto(path).map_err(|error| error.to_string())?;
    let range = workbook
        .worksheet_range_at(0)
        .ok_or_else(|| "فایل اکسل هیچ برگه‌ای ندارد".to_owned())?
        .map_err(|error| error.to_string())?;
    let mut rows = range.rows();
    let header = rows.next().ok_or_else(|| "فایل اکسل خالی است".to_owned())?;
    for (index, expected) in HEADERS.iter().enumerate() {
        if cell_text(header.get(index)).trim() != *expected {
            return Err(format!("ستون {} باید «{}» باشد", index + 1, expected));
        }
    }
    let mut result = InventoryImportFile {
        rows: Vec::new(),
        errors: Vec::new(),
    };
    for (offset, row) in rows.enumerate() {
        let number = offset + 2;
        let values: Vec<String> = (0..5).map(|index| cell_text(row.get(index))).collect();
        if values.iter().all(|value| value.trim().is_empty()) {
            continue;
        }
        match parse_row(&values) {
            Some(value) => result.rows.push(value),
            None => result.errors.push(format!("ردیف {number} نامعتبر است")),
        }
    }
    Ok(result)
}

fn parse_row(values: &[String]) -> Option<InventoryImportRow> {
    let warehouse = values[0].trim();
    let sku = values[1].trim();
    let quantity = parse_decimal(&values[2])?;
    let cost = parse_integer(&values[3])?;
    if warehouse.is_empty() || sku.is_empty() || quantity <= 0 {
        return None;
    }
    Some(InventoryImportRow {
        warehouse: warehouse.into(),
        sku: sku.into(),
        quantity_milliunits: quantity,
        unit_cost_minor: cost,
        reference: (!values[4].trim().is_empty()).then(|| values[4].trim().to_owned()),
    })
}

fn cell_text(cell: Option<&Data>) -> String {
    cell.map(ToString::to_string).unwrap_or_default()
}

fn normalize_digits(value: &str, decimal: bool) -> Option<String> {
    let value: String = value
        .chars()
        .filter_map(|character| match character {
            '0'..='9' => Some(character),
            '۰'..='۹' => char::from_digit(character as u32 - '۰' as u32, 10),
            '٠'..='٩' => char::from_digit(character as u32 - '٠' as u32, 10),
            '.' | '٫' | '/' if decimal => Some('.'),
            ',' | '٬' | ' ' => None,
            _ => Some('\0'),
        })
        .collect();
    (!value.is_empty() && !value.contains('\0')).then_some(value)
}

fn parse_integer(value: &str) -> Option<i64> {
    normalize_digits(value, false)?.parse().ok()
}

fn parse_decimal(value: &str) -> Option<i64> {
    let value = normalize_digits(value, true)?;
    let (whole, fraction) = value.split_once('.').unwrap_or((&value, ""));
    if fraction.len() > 3 || fraction.contains('.') {
        return None;
    }
    let whole = if whole.is_empty() {
        0
    } else {
        whole.parse::<i64>().ok()?
    };
    whole
        .checked_mul(1_000)?
        .checked_add(format!("{fraction:0<3}").parse().ok()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_inventory_template_can_be_read() {
        let path = std::env::temp_dir().join(format!(
            "biztrace-inventory-template-{}.xlsx",
            std::process::id()
        ));
        write_inventory_template(&path).unwrap();
        let file = read_inventory_excel(&path).unwrap();
        std::fs::remove_file(path).unwrap();
        assert_eq!(file.rows.len(), 2);
        assert_eq!(file.rows[0].quantity_milliunits, 100_000);
        assert_eq!(file.rows[0].unit_cost_minor, 40_000_000);
    }

    #[test]
    fn inventory_export_can_be_imported_again() {
        let path = std::env::temp_dir().join(format!(
            "biztrace-inventory-export-{}.xlsx",
            std::process::id()
        ));
        write_inventory_export(
            &path,
            &[StockRecord {
                cost_layer_id: 1,
                warehouse_id: 1,
                warehouse_name: "انبار مرکزی".into(),
                item_id: 1,
                item_name: "قهوه".into(),
                sku: "COF-1".into(),
                unit: "kilogram".into(),
                quantity_milliunits: 12_500,
                acquired_quantity_milliunits: 12_500,
                inventory_value_minor: 500_000_000,
                unit_cost_minor: 40_000_000,
            }],
        )
        .unwrap();
        let file = read_inventory_excel(&path).unwrap();
        std::fs::remove_file(path).unwrap();
        assert_eq!(file.rows.len(), 1);
        assert_eq!(file.rows[0].quantity_milliunits, 12_500);
        assert_eq!(file.rows[0].sku, "COF-1");
    }
}
