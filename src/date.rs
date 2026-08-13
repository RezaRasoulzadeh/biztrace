use std::time::{SystemTime, UNIX_EPOCH};

pub fn today_gregorian() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
        + 12_600;
    let (year, month, day) = civil_from_days(seconds.div_euclid(86_400));
    format!("{year:04}-{month:02}-{day:02}")
}

pub fn jalali_component(value: &str, component: i32) -> i32 {
    let parts = value
        .split('-')
        .filter_map(|part| part.parse::<i32>().ok())
        .collect::<Vec<_>>();
    if parts.len() != 3 {
        return 0;
    }
    let Some(jalali) = gregorian_to_jalali(parts[0], parts[1], parts[2]) else {
        return 0;
    };
    jalali
        .split('/')
        .nth(component.max(0) as usize)
        .and_then(|part| part.parse().ok())
        .unwrap_or(0)
}

fn civil_from_days(days_since_epoch: i64) -> (i32, i32, i32) {
    let z = days_since_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 }.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let mut year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    (year as i32, month as i32, day as i32)
}

pub fn jalali_to_gregorian(jy: i32, jm: i32, jd: i32) -> Option<String> {
    if !(1..=12).contains(&jm) || jd < 1 || jd > jalali_days_in_month(jy, jm) {
        return None;
    }
    let jy2 = jy - 979;
    let jm2 = jm - 1;
    let jd2 = jd - 1;
    let mut j_day_no = 365 * jy2 + (jy2 / 33) * 8 + ((jy2 % 33 + 3) / 4);
    for month in 0..jm2 {
        j_day_no += if month < 6 { 31 } else { 30 };
    }
    j_day_no += jd2;
    let mut g_day_no = j_day_no + 79;
    let mut gy = 1600 + 400 * (g_day_no / 146097);
    g_day_no %= 146097;
    let mut leap = true;
    if g_day_no >= 36525 {
        g_day_no -= 1;
        gy += 100 * (g_day_no / 36524);
        g_day_no %= 36524;
        if g_day_no >= 365 {
            g_day_no += 1;
        } else {
            leap = false;
        }
    }
    gy += 4 * (g_day_no / 1461);
    g_day_no %= 1461;
    if g_day_no >= 366 {
        leap = false;
        g_day_no -= 1;
        gy += g_day_no / 365;
        g_day_no %= 365;
    }
    let months = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut gm = 0;
    while gm < 12 && g_day_no >= months[gm] {
        g_day_no -= months[gm];
        gm += 1;
    }
    Some(format!("{gy:04}-{:02}-{:02}", gm + 1, g_day_no + 1))
}

pub fn jalali_days_in_month(year: i32, month: i32) -> i32 {
    if month <= 6 {
        31
    } else if month <= 11 {
        30
    } else if is_jalali_leap(year) {
        30
    } else {
        29
    }
}
pub fn gregorian_to_jalali(gy: i32, gm: i32, gd: i32) -> Option<String> {
    if crate::models::Date::new(gy, gm as u8, gd as u8).is_err() {
        return None;
    }
    let months = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let gy2 = gy - 1600;
    let gm2 = gm - 1;
    let gd2 = gd - 1;
    let mut g_day_no = 365 * gy2 + (gy2 + 3) / 4 - (gy2 + 99) / 100 + (gy2 + 399) / 400;
    for i in 0..gm2 {
        g_day_no += months[i as usize]
    }
    if gm2 > 1 && ((gy2 % 4 == 0 && gy2 % 100 != 0) || gy2 % 400 == 0) {
        g_day_no += 1
    }
    g_day_no += gd2;
    let mut j_day_no = g_day_no - 79;
    let j_np = j_day_no / 12053;
    j_day_no %= 12053;
    let mut jy = 979 + 33 * j_np + 4 * (j_day_no / 1461);
    j_day_no %= 1461;
    if j_day_no >= 366 {
        jy += (j_day_no - 1) / 365;
        j_day_no = (j_day_no - 1) % 365
    }
    let mut jm = 1;
    while jm <= 11 && j_day_no >= if jm <= 6 { 31 } else { 30 } {
        j_day_no -= if jm <= 6 { 31 } else { 30 };
        jm += 1
    }
    Some(format!("{jy:04}/{jm:02}/{:02}", j_day_no + 1))
}
pub fn parse_jalali(value: &str) -> Option<String> {
    let normalized = value.replace('-', "/");
    let p = normalized.split('/').collect::<Vec<_>>();
    if p.len() != 3 {
        return None;
    }
    jalali_to_gregorian(p[0].parse().ok()?, p[1].parse().ok()?, p[2].parse().ok()?)
}
pub fn jalali_calendar_day(year: i32, month: i32, cell: i32) -> i32 {
    let Some(first) = jalali_to_gregorian(year, month, 1) else {
        return 0;
    };
    let p = first
        .split('-')
        .filter_map(|v| v.parse::<i32>().ok())
        .collect::<Vec<_>>();
    if p.len() != 3 {
        return 0;
    }
    let mut y = p[0];
    let mut m = p[1];
    if m < 3 {
        y -= 1;
        m += 12
    }
    let k = y % 100;
    let j = y / 100;
    let h = (p[2] + (13 * (m + 1)) / 5 + k + k / 4 + j / 4 + 5 * j) % 7;
    let day = cell - h + 1;
    if day >= 1 && day <= jalali_days_in_month(year, month) {
        day
    } else {
        0
    }
}
fn is_jalali_leap(year: i32) -> bool {
    let base = if year > 474 { 473 } else { 474 };
    ((((year - base) % 2820 + 474 + 38) * 682) % 2816) < 682
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn converts_known_nowruz_dates() {
        assert_eq!(
            jalali_to_gregorian(1403, 1, 1).as_deref(),
            Some("2024-03-20")
        );
        assert_eq!(
            jalali_to_gregorian(1404, 1, 1).as_deref(),
            Some("2025-03-21")
        );
        assert_eq!(
            gregorian_to_jalali(2024, 3, 20).as_deref(),
            Some("1403/01/01")
        );
    }
}
