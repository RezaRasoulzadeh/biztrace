use crate::database::{CustomerDraft, CustomerRecord};
use calamine::{Data, Reader, open_workbook_auto};
use rust_xlsxwriter::Workbook;
use std::path::Path;

const HEADERS: [&str; 7] = [
    "نوع",
    "نام",
    "تلفن",
    "ایمیل",
    "آدرس",
    "نوع مانده",
    "مانده (ریال)",
];
pub struct CustomerImportRow {
    pub customer: CustomerDraft,
    pub opening_balance_minor: i64,
}
pub struct CustomerImportFile {
    pub rows: Vec<CustomerImportRow>,
    pub errors: Vec<String>,
}

pub fn write_customer_template(path: &Path) -> Result<(), String> {
    write_workbook(
        path,
        "مشتریان",
        &[
            (
                "حقیقی",
                "علی رضایی",
                "09120000000",
                "ali@example.com",
                "تهران",
                "بدهکار",
                2_500_000,
            ),
            (
                "حقوقی",
                "شرکت نمونه",
                "02100000000",
                "info@example.com",
                "تهران",
                "بستانکار",
                1_000_000,
            ),
        ],
    )
}
pub fn write_customer_export(path: &Path, records: &[CustomerRecord]) -> Result<(), String> {
    let rows = records
        .iter()
        .map(|item| {
            let (balance_type, amount) = if item.balance_minor > 0 {
                ("بدهکار", item.balance_minor)
            } else if item.balance_minor < 0 {
                ("بستانکار", item.balance_minor.abs())
            } else {
                ("تسویه", 0)
            };
            (
                if item.kind == "individual" {
                    "حقیقی"
                } else {
                    "حقوقی"
                },
                item.name.as_str(),
                item.phone.as_str(),
                item.email.as_str(),
                item.address.as_str(),
                balance_type,
                amount,
            )
        })
        .collect::<Vec<_>>();
    write_workbook(path, "خروجی مشتریان", &rows)
}
fn write_workbook(
    path: &Path,
    name: &str,
    rows: &[(&str, &str, &str, &str, &str, &str, i64)],
) -> Result<(), String> {
    let mut workbook = Workbook::new();
    let sheet = workbook.add_worksheet();
    sheet.set_name(name).map_err(|e| e.to_string())?;
    sheet.set_right_to_left(true);
    for (column, header) in HEADERS.iter().enumerate() {
        sheet
            .write_string(0, column as u16, *header)
            .map_err(|e| e.to_string())?;
    }
    for (index, row) in rows.iter().enumerate() {
        for (column, value) in [row.0, row.1, row.2, row.3, row.4, row.5]
            .iter()
            .enumerate()
        {
            sheet
                .write_string((index + 1) as u32, column as u16, *value)
                .map_err(|e| e.to_string())?;
        }
        sheet
            .write_number((index + 1) as u32, 6, row.6 as f64)
            .map_err(|e| e.to_string())?;
    }
    for (column, width) in [12., 28., 18., 28., 35., 14., 20.].iter().enumerate() {
        sheet
            .set_column_width(column as u16, *width)
            .map_err(|e| e.to_string())?;
    }
    workbook.save(path).map_err(|e| e.to_string())
}

pub fn read_customer_excel(path: &Path) -> Result<CustomerImportFile, String> {
    let mut workbook = open_workbook_auto(path).map_err(|e| e.to_string())?;
    let range = workbook
        .worksheet_range_at(0)
        .ok_or_else(|| "فایل اکسل هیچ برگه‌ای ندارد".to_owned())?
        .map_err(|e| e.to_string())?;
    let mut rows = range.rows();
    let header = rows.next().ok_or_else(|| "فایل اکسل خالی است".to_owned())?;
    for (index, expected) in HEADERS.iter().enumerate() {
        if cell_text(header.get(index)).trim() != *expected {
            return Err(format!("ستون {} باید «{}» باشد", index + 1, expected));
        }
    }
    let mut result = CustomerImportFile {
        rows: vec![],
        errors: vec![],
    };
    for (offset, row) in rows.enumerate() {
        let values = (0..7)
            .map(|index| cell_text(row.get(index)))
            .collect::<Vec<_>>();
        if values.iter().all(|value| value.trim().is_empty()) {
            continue;
        }
        match parse_row(&values, offset + 2) {
            Ok(row) => result.rows.push(row),
            Err(error) => result.errors.push(error),
        }
    }
    if result.rows.is_empty() && result.errors.is_empty() {
        return Err("هیچ ردیف قابل ورودی در فایل پیدا نشد".into());
    }
    Ok(result)
}
fn parse_row(values: &[String], row: usize) -> Result<CustomerImportRow, String> {
    let kind = match values[0].trim() {
        "حقیقی" | "individual" => "individual",
        "حقوقی" | "business" => "business",
        _ => return Err(format!("ردیف {row}: نوع مشتری معتبر نیست")),
    };
    let name = values[1].trim();
    if name.is_empty() {
        return Err(format!("ردیف {row}: نام خالی است"));
    }
    let amount =
        parse_amount(&values[6]).ok_or_else(|| format!("ردیف {row}: مبلغ مانده معتبر نیست"))?;
    let opening_balance_minor = match values[5].trim() {
        "بدهکار" | "debit" => amount,
        "بستانکار" | "credit" => -amount,
        "تسویه" | "settled" => 0,
        _ => return Err(format!("ردیف {row}: نوع مانده معتبر نیست")),
    };
    Ok(CustomerImportRow {
        customer: CustomerDraft {
            id: None,
            kind: kind.into(),
            name: name.into(),
            phone: optional(&values[2]),
            email: optional(&values[3]),
            address: optional(&values[4]),
        },
        opening_balance_minor,
    })
}
fn optional(value: &str) -> Option<String> {
    (!value.trim().is_empty()).then(|| value.trim().to_owned())
}
fn cell_text(cell: Option<&Data>) -> String {
    cell.map(ToString::to_string).unwrap_or_default()
}
fn parse_amount(value: &str) -> Option<i64> {
    let normalized = value
        .chars()
        .filter_map(|c| match c {
            '0'..='9' => Some(c),
            '۰'..='۹' => char::from_digit(c as u32 - '۰' as u32, 10),
            '٠'..='٩' => char::from_digit(c as u32 - '٠' as u32, 10),
            ',' | '٬' | ' ' => None,
            _ => Some('\0'),
        })
        .collect::<String>();
    if normalized.is_empty() || normalized.contains('\0') {
        None
    } else {
        normalized.parse().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn customer_template_round_trips_with_balances() {
        let path = std::env::temp_dir().join(format!(
            "biztrace-customer-template-{}.xlsx",
            std::process::id()
        ));
        write_customer_template(&path).unwrap();
        let file = read_customer_excel(&path).unwrap();
        std::fs::remove_file(path).unwrap();
        assert_eq!(file.rows.len(), 2);
        assert_eq!(file.rows[1].customer.kind, "business");
        assert_eq!(file.rows[0].opening_balance_minor, 2_500_000);
        assert_eq!(file.rows[1].opening_balance_minor, -1_000_000);
    }
}
