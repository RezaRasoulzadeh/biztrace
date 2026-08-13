use crate::{
    database::{FundAccountRecord, FundCheckRecord, FundTransactionRecord},
    date::{gregorian_to_jalali, parse_jalali},
};
use calamine::{Data, Reader, open_workbook_auto};
use rust_xlsxwriter::Workbook;
use std::path::Path;

pub enum FundImportFile {
    Accounts(Vec<AccountImport>),
    Transactions(Vec<TransactionImport>),
    Checks(Vec<CheckImport>),
}
pub struct AccountImport {
    pub kind: String,
    pub name: String,
    pub number: Option<String>,
    pub opening: i64,
}
pub struct TransactionImport {
    pub kind: String,
    pub account: String,
    pub target: Option<String>,
    pub amount: i64,
    pub category: String,
    pub date: String,
    pub reference: Option<String>,
    pub description: Option<String>,
}
pub struct CheckImport {
    pub direction: String,
    pub account: String,
    pub party: String,
    pub number: String,
    pub bank: Option<String>,
    pub amount: i64,
    pub due: String,
    pub note: Option<String>,
}
const AH: [&str; 4] = [
    "نوع حساب",
    "نام حساب",
    "شماره/شناسه",
    "موجودی افتتاحیه (ریال)",
];
const TH: [&str; 8] = [
    "نوع",
    "حساب",
    "حساب مقصد",
    "مبلغ (ریال)",
    "دسته‌بندی",
    "تاریخ شمسی",
    "شناسه پیگیری",
    "شرح",
];
const CH: [&str; 8] = [
    "جهت",
    "حساب مرتبط",
    "طرف حساب",
    "شماره چک",
    "بانک صادرکننده",
    "مبلغ (ریال)",
    "سررسید شمسی",
    "یادداشت",
];

pub fn write_fund_template(path: &Path, tab: i32) -> Result<(), String> {
    let mut w = Workbook::new();
    let s = w.add_worksheet();
    s.set_right_to_left(true);
    let headers: &[&str] = match tab {
        0 => &AH,
        1 => &TH,
        _ => &CH,
    };
    for (i, h) in headers.iter().enumerate() {
        s.write_string(0, i as u16, *h).map_err(|e| e.to_string())?;
    }
    let sample = match tab {
        0 => vec![
            "حساب بانکی",
            "بانک ملت",
            "IR000000000000000000000000",
            "10000000",
        ],
        1 => vec![
            "درآمد",
            "بانک ملت",
            "",
            "2500000",
            "فروش",
            "1405/05/22",
            "TRX-1001",
            "فروش نقدی",
        ],
        _ => vec![
            "دریافتی",
            "بانک ملت",
            "شرکت نمونه",
            "CHK-1001",
            "بانک سامان",
            "5000000",
            "1405/06/15",
            "فاکتور ۱۲۳",
        ],
    };
    for (i, v) in sample.iter().enumerate() {
        s.write_string(1, i as u16, *v).map_err(|e| e.to_string())?;
    }
    w.save(path).map_err(|e| e.to_string())
}
pub fn write_fund_export(
    path: &Path,
    tab: i32,
    accounts: &[FundAccountRecord],
    transactions: &[FundTransactionRecord],
    checks: &[FundCheckRecord],
) -> Result<(), String> {
    let mut w = Workbook::new();
    let s = w.add_worksheet();
    s.set_right_to_left(true);
    let headers: &[&str] = match tab {
        0 => &AH,
        1 => &TH,
        _ => &CH,
    };
    for (i, h) in headers.iter().enumerate() {
        s.write_string(0, i as u16, *h).map_err(|e| e.to_string())?;
    }
    match tab {
        0 => {
            for (i, r) in accounts.iter().enumerate() {
                let row = (i + 1) as u32;
                for (c, v) in [
                    kind_label(&r.kind),
                    r.name.as_str(),
                    r.account_number.as_str(),
                ]
                .iter()
                .enumerate()
                {
                    s.write_string(row, c as u16, *v)
                        .map_err(|e| e.to_string())?;
                }
                s.write_number(row, 3, r.opening_balance_minor as f64)
                    .map_err(|e| e.to_string())?;
            }
        }
        1 => {
            for (i, r) in transactions.iter().enumerate() {
                let row = (i + 1) as u32;
                let date = jalali(&r.occurred_on);
                for (c, v) in [
                    transaction_label(&r.kind),
                    r.account_name.as_str(),
                    r.transfer_account_name.as_str(),
                    "",
                    r.category.as_str(),
                    date.as_str(),
                    r.reference.as_str(),
                    r.description.as_str(),
                ]
                .iter()
                .enumerate()
                {
                    if c == 3 {
                        s.write_number(row, c as u16, r.amount_minor as f64)
                            .map_err(|e| e.to_string())?;
                    } else {
                        s.write_string(row, c as u16, *v)
                            .map_err(|e| e.to_string())?;
                    }
                }
            }
        }
        _ => {
            for (i, r) in checks.iter().enumerate() {
                let row = (i + 1) as u32;
                let due = jalali(&r.due_on);
                for (c, v) in [
                    if r.direction == "incoming" {
                        "دریافتی"
                    } else {
                        "پرداختی"
                    },
                    r.account_name.as_str(),
                    r.party_name.as_str(),
                    r.check_number.as_str(),
                    r.bank_name.as_str(),
                    "",
                    due.as_str(),
                    r.note.as_str(),
                ]
                .iter()
                .enumerate()
                {
                    if c == 5 {
                        s.write_number(row, c as u16, r.amount_minor as f64)
                            .map_err(|e| e.to_string())?;
                    } else {
                        s.write_string(row, c as u16, *v)
                            .map_err(|e| e.to_string())?;
                    }
                }
            }
        }
    }
    w.save(path).map_err(|e| e.to_string())
}
pub fn read_fund_excel(path: &Path, tab: i32) -> Result<FundImportFile, String> {
    let mut w = open_workbook_auto(path).map_err(|e| e.to_string())?;
    let range = w
        .worksheet_range_at(0)
        .ok_or("فایل بدون برگه است")?
        .map_err(|e| e.to_string())?;
    let mut rows = range.rows();
    let header = rows.next().ok_or("فایل خالی است")?;
    let expected: &[&str] = match tab {
        0 => &AH,
        1 => &TH,
        _ => &CH,
    };
    for (i, h) in expected.iter().enumerate() {
        if text(header.get(i)).trim() != *h {
            return Err(format!("ستون {} باید «{}» باشد", i + 1, h));
        }
    }
    if tab == 0 {
        let mut out = vec![];
        for r in rows {
            let v = (0..4).map(|i| text(r.get(i))).collect::<Vec<_>>();
            if v.iter().all(|x| x.trim().is_empty()) {
                continue;
            }
            out.push(AccountImport {
                kind: kind_code(&v[0])?.into(),
                name: required(&v[1])?,
                number: optional(&v[2]),
                opening: amount(&v[3])?,
            });
        }
        return Ok(FundImportFile::Accounts(out));
    }
    if tab == 1 {
        let mut out = vec![];
        for r in rows {
            let v = (0..8).map(|i| text(r.get(i))).collect::<Vec<_>>();
            if v.iter().all(|x| x.trim().is_empty()) {
                continue;
            }
            out.push(TransactionImport {
                kind: transaction_code(&v[0])?.into(),
                account: required(&v[1])?,
                target: optional(&v[2]),
                amount: amount(&v[3])?,
                category: required(&v[4])?,
                date: parse_jalali(v[5].trim()).ok_or("تاریخ شمسی نامعتبر است")?,
                reference: optional(&v[6]),
                description: optional(&v[7]),
            });
        }
        return Ok(FundImportFile::Transactions(out));
    }
    let mut out = vec![];
    for r in rows {
        let v = (0..8).map(|i| text(r.get(i))).collect::<Vec<_>>();
        if v.iter().all(|x| x.trim().is_empty()) {
            continue;
        }
        let direction = match v[0].trim() {
            "دریافتی" => "incoming",
            "پرداختی" => "outgoing",
            _ => return Err("جهت چک نامعتبر است".into()),
        };
        out.push(CheckImport {
            direction: direction.into(),
            account: required(&v[1])?,
            party: required(&v[2])?,
            number: required(&v[3])?,
            bank: optional(&v[4]),
            amount: amount(&v[5])?,
            due: parse_jalali(v[6].trim()).ok_or("سررسید شمسی نامعتبر است")?,
            note: optional(&v[7]),
        });
    }
    Ok(FundImportFile::Checks(out))
}
fn text(v: Option<&Data>) -> String {
    v.map(ToString::to_string).unwrap_or_default()
}
fn optional(v: &str) -> Option<String> {
    (!v.trim().is_empty()).then(|| v.trim().into())
}
fn required(v: &str) -> Result<String, String> {
    optional(v).ok_or("فیلد الزامی خالی است".into())
}
fn amount(v: &str) -> Result<i64, String> {
    v.replace([',', '٬', ' '], "")
        .parse()
        .map_err(|_| "مبلغ نامعتبر است".into())
}
fn kind_code(v: &str) -> Result<&'static str, String> {
    match v.trim() {
        "صندوق نقدی" => Ok("cash"),
        "حساب بانکی" => Ok("bank"),
        "کارت" => Ok("card"),
        "سایر" => Ok("other"),
        _ => Err("نوع حساب نامعتبر است".into()),
    }
}
fn transaction_code(v: &str) -> Result<&'static str, String> {
    match v.trim() {
        "درآمد" => Ok("income"),
        "هزینه" => Ok("expense"),
        "انتقال" => Ok("transfer"),
        _ => Err("نوع تراکنش نامعتبر است".into()),
    }
}
fn kind_label(v: &str) -> &str {
    match v {
        "cash" => "صندوق نقدی",
        "bank" => "حساب بانکی",
        "card" => "کارت",
        _ => "سایر",
    }
}
fn transaction_label(v: &str) -> &str {
    match v {
        "income" => "درآمد",
        "expense" => "هزینه",
        _ => "انتقال",
    }
}
fn jalali(v: &str) -> String {
    let p = v
        .split('-')
        .filter_map(|x| x.parse().ok())
        .collect::<Vec<i32>>();
    if p.len() == 3 {
        gregorian_to_jalali(p[0], p[1], p[2]).unwrap_or_default()
    } else {
        String::new()
    }
}
