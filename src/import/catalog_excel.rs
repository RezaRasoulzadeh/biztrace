// src/import/catalog_excel.rs

use std::path::Path;

use calamine::{Data, Reader, open_workbook_auto};
use rust_xlsxwriter::Workbook;

use crate::database::{CatalogDraft, CatalogRecord};

const HEADERS: [&str; 5] = ["نوع", "نام", "کد کالا", "واحد", "قیمت فروش (ریال)"];

pub struct CatalogImportFile {
    pub rows: Vec<CatalogDraft>,
    pub errors: Vec<String>,
}

pub fn write_catalog_template(path: &Path) -> Result<(), String> {
    let mut workbook = Workbook::new();
    let sheet = workbook.add_worksheet();
    sheet
        .set_name("کالاها و خدمات")
        .map_err(|error| error.to_string())?;
    sheet.set_right_to_left(true);
    for (column, header) in HEADERS.iter().enumerate() {
        sheet
            .write_string(0, column as u16, *header)
            .map_err(|error| error.to_string())?;
    }
    let samples = [
        ["کالا", "اسپرسو", "DRK-ESPRESSO", "عدد", "1200000"],
        ["کالا", "لاته", "DRK-LATTE", "عدد", "1800000"],
        [
            "کالا",
            "دانه قهوه عربیکا",
            "COF-ARABICA-1KG",
            "کیلوگرم",
            "9500000",
        ],
        ["خدمت", "رزرو میز ویژه", "SRV-VIP", "ساعت", "3000000"],
    ];
    for (row, values) in samples.iter().enumerate() {
        for (column, value) in values.iter().enumerate() {
            sheet
                .write_string((row + 1) as u32, column as u16, *value)
                .map_err(|error| error.to_string())?;
        }
    }
    sheet
        .set_column_width(0, 12)
        .map_err(|error| error.to_string())?;
    sheet
        .set_column_width(1, 28)
        .map_err(|error| error.to_string())?;
    sheet
        .set_column_width(2, 18)
        .map_err(|error| error.to_string())?;
    sheet
        .set_column_width(3, 14)
        .map_err(|error| error.to_string())?;
    sheet
        .set_column_width(4, 22)
        .map_err(|error| error.to_string())?;
    workbook.save(path).map_err(|error| error.to_string())
}

pub fn write_catalog_export(path: &Path, records: &[CatalogRecord]) -> Result<(), String> {
    let mut workbook = Workbook::new();
    let sheet = workbook.add_worksheet();
    sheet
        .set_name("خروجی کالاها")
        .map_err(|error| error.to_string())?;
    sheet.set_right_to_left(true);
    for (column, header) in ["نوع", "نام", "کد کالا", "واحد", "قیمت فروش (ریال)"]
        .iter()
        .enumerate()
    {
        sheet
            .write_string(0, column as u16, *header)
            .map_err(|error| error.to_string())?;
    }
    for (index, record) in records.iter().enumerate() {
        let row = (index + 1) as u32;
        sheet
            .write_string(
                row,
                0,
                if record.kind == "product" {
                    "کالا"
                } else {
                    "خدمت"
                },
            )
            .map_err(|error| error.to_string())?;
        sheet
            .write_string(row, 1, &record.name)
            .map_err(|error| error.to_string())?;
        sheet
            .write_string(row, 2, &record.sku)
            .map_err(|error| error.to_string())?;
        sheet
            .write_string(row, 3, unit_label(&record.unit))
            .map_err(|error| error.to_string())?;
        sheet
            .write_number(row, 4, record.sale_price_minor as f64)
            .map_err(|error| error.to_string())?;
    }
    workbook.save(path).map_err(|error| error.to_string())
}

fn unit_label(value: &str) -> &str {
    match value {
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

pub fn read_catalog_excel(path: &Path) -> Result<CatalogImportFile, String> {
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
    let mut result = CatalogImportFile {
        rows: Vec::new(),
        errors: Vec::new(),
    };
    for (offset, row) in rows.enumerate() {
        let row_number = offset + 2;
        let values: Vec<String> = (0..5).map(|index| cell_text(row.get(index))).collect();
        if values.iter().all(|value| value.trim().is_empty()) {
            continue;
        }
        match parse_row(&values, row_number) {
            Ok(draft) => {
                result.rows.push(draft);
            }
            Err(error) => result.errors.push(error),
        }
    }
    if result.rows.is_empty() && result.errors.is_empty() {
        return Err("هیچ ردیف قابل ورودی در فایل پیدا نشد".into());
    }
    Ok(result)
}

fn parse_row(values: &[String], row: usize) -> Result<CatalogDraft, String> {
    let kind = match values[0].trim() {
        "کالا" | "product" => "product",
        "خدمت" | "service" => "service",
        _ => return Err(format!("ردیف {row}: نوع باید کالا یا خدمت باشد")),
    };
    let name = values[1].trim();
    if name.is_empty() {
        return Err(format!("ردیف {row}: نام خالی است"));
    }
    let unit = unit_code(values[3].trim()).ok_or_else(|| format!("ردیف {row}: واحد معتبر نیست"))?;
    let price =
        parse_price(&values[4]).ok_or_else(|| format!("ردیف {row}: قیمت فروش معتبر نیست"))?;
    Ok(CatalogDraft {
        id: None,
        kind: kind.into(),
        name: name.into(),
        sku: (!values[2].trim().is_empty()).then(|| values[2].trim().to_owned()),
        unit: unit.into(),
        sale_price_minor: price,
    })
}

fn cell_text(cell: Option<&Data>) -> String {
    cell.map(ToString::to_string).unwrap_or_default()
}

fn parse_price(value: &str) -> Option<i64> {
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

fn unit_code(value: &str) -> Option<&'static str> {
    match value {
        "عدد" | "each" => Some("each"),
        "کیلوگرم" | "kilogram" => Some("kilogram"),
        "گرم" | "gram" => Some("gram"),
        "لیتر" | "liter" => Some("liter"),
        "متر" | "meter" => Some("meter"),
        "ساعت" | "hour" => Some("hour"),
        "جلسه" | "session" => Some("session"),
        "سفارشی" | "custom" => Some("custom"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_template_can_be_read_back() {
        let path = std::env::temp_dir().join(format!(
            "biztrace-catalog-template-{}.xlsx",
            std::process::id()
        ));
        write_catalog_template(&path).unwrap();
        let file = read_catalog_excel(&path).unwrap();
        std::fs::remove_file(path).unwrap();

        assert_eq!(file.rows.len(), 4);
        assert_eq!(file.rows[0].sale_price_minor, 1_200_000);
        assert_eq!(file.rows[3].kind, "service");
    }

    #[test]
    fn catalog_export_can_be_imported_again() {
        let path = std::env::temp_dir().join(format!(
            "biztrace-catalog-export-{}.xlsx",
            std::process::id()
        ));
        write_catalog_export(
            &path,
            &[CatalogRecord {
                id: 1,
                kind: "product".into(),
                name: "قهوه".into(),
                sku: "COF-1".into(),
                unit: "kilogram".into(),
                sale_price_minor: 50_000_000,
            }],
        )
        .unwrap();
        let file = read_catalog_excel(&path).unwrap();
        std::fs::remove_file(path).unwrap();
        assert_eq!(file.rows.len(), 1);
        assert_eq!(file.rows[0].sku.as_deref(), Some("COF-1"));
        assert_eq!(file.rows[0].unit, "kilogram");
    }
}
