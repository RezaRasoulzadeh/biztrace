// src/models/common.rs

use std::fmt;
use std::ops::{Add, Sub};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelError {
    EmptyField(&'static str),
    InvalidDate,
    InvalidQuantity,
    CurrencyMismatch,
    ArithmeticOverflow,
}

impl fmt::Display for ModelError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyField(field) => write!(formatter, "{field} cannot be empty"),
            Self::InvalidDate => formatter.write_str("date is invalid"),
            Self::InvalidQuantity => formatter.write_str("quantity must be greater than zero"),
            Self::CurrencyMismatch => formatter.write_str("currencies do not match"),
            Self::ArithmeticOverflow => formatter.write_str("numeric operation overflowed"),
        }
    }
}

impl std::error::Error for ModelError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Currency {
    Irr,
    Usd,
    Eur,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Money {
    pub minor_units: i64,
    pub currency: Currency,
}

impl Money {
    pub const fn new(minor_units: i64, currency: Currency) -> Self {
        Self {
            minor_units,
            currency,
        }
    }

    pub const fn zero(currency: Currency) -> Self {
        Self::new(0, currency)
    }

    pub fn checked_add(self, other: Self) -> Result<Self, ModelError> {
        if self.currency != other.currency {
            return Err(ModelError::CurrencyMismatch);
        }
        self.minor_units
            .checked_add(other.minor_units)
            .map(|value| Self::new(value, self.currency))
            .ok_or(ModelError::ArithmeticOverflow)
    }

    pub fn checked_sub(self, other: Self) -> Result<Self, ModelError> {
        if self.currency != other.currency {
            return Err(ModelError::CurrencyMismatch);
        }
        self.minor_units
            .checked_sub(other.minor_units)
            .map(|value| Self::new(value, self.currency))
            .ok_or(ModelError::ArithmeticOverflow)
    }
}

impl Add for Money {
    type Output = Result<Self, ModelError>;

    fn add(self, rhs: Self) -> Self::Output {
        self.checked_add(rhs)
    }
}

impl Sub for Money {
    type Output = Result<Self, ModelError>;

    fn sub(self, rhs: Self) -> Self::Output {
        self.checked_sub(rhs)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Quantity(i64);

impl Quantity {
    pub const SCALE: i64 = 1_000;

    pub fn from_milliunits(value: i64) -> Result<Self, ModelError> {
        if value <= 0 {
            return Err(ModelError::InvalidQuantity);
        }
        Ok(Self(value))
    }

    pub const fn milliunits(self) -> i64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Date {
    pub year: i32,
    pub month: u8,
    pub day: u8,
}

impl Date {
    pub fn new(year: i32, month: u8, day: u8) -> Result<Self, ModelError> {
        let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
        let max_day = match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 if leap => 29,
            2 => 28,
            _ => return Err(ModelError::InvalidDate),
        };
        if day == 0 || day > max_day {
            return Err(ModelError::InvalidDate);
        }
        Ok(Self { year, month, day })
    }
}
