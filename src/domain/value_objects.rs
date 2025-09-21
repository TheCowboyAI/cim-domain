// Copyright 2025 Cowboy AI, LLC.

//! Canonical domain Value Objects (invariants) used across domains.
//!
//! Value Objects are immutable, compared by value, and updated by replacement.
//! Examples:
//! - PhysicalAddress: street, locality, region, optional subregion, country, postal code
//! - Temperature: numeric value with a scale (C/F/K) — number alone is ambiguous

use crate::concepts::HasConcept;
use crate::formal_domain::{DomainConcept, ValueObject};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Currency types to classify monetary units.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum CurrencyType {
    /// Government-issued fiat currency (ISO-4217)
    Fiat {
        /// ISO 3166 country code (e.g., "US")
        country: String,
    },
    /// Crypto currency identified by blockchain/network
    Crypto {
        /// Blockchain/network identifier (e.g., "bitcoin")
        chain: String,
    },
    /// Other or virtual currency (e.g., loyalty points)
    Other {
        /// Human-readable description of currency type
        description: String,
    },
}

/// Currency definition: code and type with exponent for minor units.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct Currency {
    /// ISO-4217 code or ticker (e.g., "USD", "BTC")
    pub code: String,
    /// Minor unit exponent (e.g., 2 for cents, 8 for satoshis)
    pub exponent: u8,
    /// Classification
    pub kind: CurrencyType,
}

impl Currency {
    /// Create a Currency definition
    pub fn new(code: impl Into<String>, exponent: u8, kind: CurrencyType) -> Self {
        Self {
            code: code.into(),
            exponent,
            kind,
        }
    }
}

impl DomainConcept for Currency {}
impl ValueObject for Currency {}

/// Money as an immutable value object: amount in minor units + currency.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct Money {
    /// Amount in minor units (e.g., cents, satoshis)
    amount_minor: i128,
    currency: Currency,
}

impl Money {
    /// Create Money from a major unit amount (e.g., dollars) using currency exponent.
    pub fn from_major(major: i128, currency: Currency) -> Self {
        let factor = 10i128.pow(currency.exponent as u32);
        Self {
            amount_minor: major * factor,
            currency,
        }
    }

    /// Create Money from minor units directly.
    pub fn from_minor(minor: i128, currency: Currency) -> Self {
        Self {
            amount_minor: minor,
            currency,
        }
    }

    /// Get currency
    pub fn currency(&self) -> &Currency {
        &self.currency
    }

    /// Amount in minor units
    pub fn amount_minor(&self) -> i128 {
        self.amount_minor
    }

    /// Amount in major units as integer division (truncating remainder)
    pub fn amount_major_trunc(&self) -> i128 {
        let factor = 10i128.pow(self.currency.exponent as u32);
        self.amount_minor / factor
    }

    /// Add amounts if same currency.
    pub fn checked_add(&self, other: &Money) -> Result<Money, String> {
        if self.currency != other.currency {
            return Err("Currency mismatch".to_string());
        }
        Ok(Money {
            amount_minor: self.amount_minor + other.amount_minor,
            currency: self.currency.clone(),
        })
    }

    /// Subtract amounts if same currency.
    pub fn checked_sub(&self, other: &Money) -> Result<Money, String> {
        if self.currency != other.currency {
            return Err("Currency mismatch".to_string());
        }
        Ok(Money {
            amount_minor: self.amount_minor - other.amount_minor,
            currency: self.currency.clone(),
        })
    }
}

impl DomainConcept for Money {}
impl ValueObject for Money {}

impl HasConcept for Money {
    fn concept_id() -> &'static str {
        "money"
    }
}

/// Conversion event between currencies represented as a rational ratio of minor units.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct ConversionRate {
    /// Source currency
    pub from: Currency,
    /// Target currency
    pub to: Currency,
    /// Numerator of rational rate: target_minor = source_minor * num / den
    pub num: i128,
    /// Denominator of rational rate
    pub den: i128,
    /// Event timestamp (unix seconds) for ordering (latest wins)
    pub at: i64,
}

/// Provider of latest conversion rates (e.g., backed by an event log downstream).
pub trait RateProvider {
    /// Return latest rate (num, den) for converting `from` → `to` in terms of minor units.
    fn latest_rate(&self, from: &Currency, to: &Currency) -> Option<(i128, i128)>;
}

impl Money {
    /// Convert this amount to target currency using a RateProvider (pure conversion; no side effects).
    pub fn convert_to<R: RateProvider>(
        &self,
        target: &Currency,
        rates: &R,
    ) -> Result<Money, String> {
        if &self.currency == target {
            return Ok(self.clone());
        }
        let (num, den) = rates
            .latest_rate(&self.currency, target)
            .ok_or_else(|| "No conversion rate".to_string())?;
        if den == 0 {
            return Err("Invalid rate denominator 0".to_string());
        }
        // Round to nearest minor unit with symmetric rounding
        let sign = if self.amount_minor >= 0 {
            1i128
        } else {
            -1i128
        };
        let abs = self.amount_minor.abs();
        let scaled = abs.saturating_mul(num);
        let rounded = (scaled + (den / 2)) / den;
        let minor = sign * rounded;
        Ok(Money {
            amount_minor: minor,
            currency: target.clone(),
        })
    }

    /// Add two amounts in potentially different currencies by converting both to `target`.
    pub fn add_in_currency<R: RateProvider>(
        &self,
        other: &Money,
        target: &Currency,
        rates: &R,
    ) -> Result<Money, String> {
        let a = self.convert_to(target, rates)?;
        let b = other.convert_to(target, rates)?;
        Ok(Money {
            amount_minor: a.amount_minor + b.amount_minor,
            currency: target.clone(),
        })
    }
}

/// Postal address as a single invariant value.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct PhysicalAddress {
    street: String,
    locality: String,
    region: String,
    subregion: Option<String>,
    country: String,
    postal_code: String,
}

impl PhysicalAddress {
    /// Construct a physical address invariant
    pub fn new(
        street: String,
        locality: String,
        region: String,
        country: String,
        postal_code: String,
    ) -> Self {
        Self {
            street,
            locality,
            region,
            subregion: None,
            country,
            postal_code,
        }
    }

    /// Set an optional subregion (e.g., county)
    pub fn with_subregion(mut self, sub: Option<String>) -> Self {
        self.subregion = sub;
        self
    }

    // Getters keep fields encapsulated to preserve invariants
    /// Street line
    pub fn street(&self) -> &str {
        &self.street
    }
    /// City or locality
    pub fn locality(&self) -> &str {
        &self.locality
    }
    /// Region or state/province
    pub fn region(&self) -> &str {
        &self.region
    }
    /// Optional subregion
    pub fn subregion(&self) -> Option<&str> {
        self.subregion.as_deref()
    }
    /// Country code
    pub fn country(&self) -> &str {
        &self.country
    }
    /// Postal/ZIP code
    pub fn postal_code(&self) -> &str {
        &self.postal_code
    }

    // Immutable updates return new values
    /// Return a copy with new street
    pub fn with_street(mut self, street: String) -> Self {
        self.street = street;
        self
    }
    /// Return a copy with new locality
    pub fn with_locality(mut self, locality: String) -> Self {
        self.locality = locality;
        self
    }
    /// Return a copy with new region
    pub fn with_region(mut self, region: String) -> Self {
        self.region = region;
        self
    }
    /// Return a copy with new country
    pub fn with_country(mut self, country: String) -> Self {
        self.country = country;
        self
    }
    /// Return a copy with new postal code
    pub fn with_postal_code(mut self, code: String) -> Self {
        self.postal_code = code;
        self
    }
}

impl DomainConcept for PhysicalAddress {}
impl ValueObject for PhysicalAddress {}

/// Temperature scale makes numeric value meaningful.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum TemperatureScale {
    /// Degrees Celsius (°C)
    Celsius,
    /// Degrees Fahrenheit (°F)
    Fahrenheit,
    /// Kelvin (K)
    Kelvin,
}

/// Temperature: value + scale (immutable value object)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Temperature {
    /// Canonical representation in microkelvin to preserve Eq semantics
    micro_kelvin: i64,
}

impl Temperature {
    /// Construct a temperature from a numeric value and scale
    pub fn new(value: f64, scale: TemperatureScale) -> Self {
        use TemperatureScale::*;
        let kelvin = match scale {
            Celsius => value + 273.15,
            Fahrenheit => (value - 32.0) * 5.0 / 9.0 + 273.15,
            Kelvin => value,
        };
        Self {
            micro_kelvin: (kelvin * 1_000_000.0).round() as i64,
        }
    }

    /// Get the numeric value in the requested scale
    pub fn value_in(&self, scale: TemperatureScale) -> f64 {
        let kelvin = self.micro_kelvin as f64 / 1_000_000.0;
        match scale {
            TemperatureScale::Kelvin => kelvin,
            TemperatureScale::Celsius => kelvin - 273.15,
            TemperatureScale::Fahrenheit => (kelvin - 273.15) * 9.0 / 5.0 + 32.0,
        }
    }
}

impl DomainConcept for Temperature {}
impl ValueObject for Temperature {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn physical_address_is_value_object() {
        let addr1 = PhysicalAddress::new(
            "123 Main St".into(),
            "Springfield".into(),
            "IL".into(),
            "US".into(),
            "62701".into(),
        );
        let addr2 = addr1.clone();
        assert_eq!(addr1, addr2);

        // Immutable update returns a different value, original unchanged
        let addr3 = addr1.clone().with_locality("Shelbyville".into());
        assert_ne!(addr1, addr3);
        assert_eq!(addr1.locality(), "Springfield");
        assert_eq!(addr3.locality(), "Shelbyville");
    }

    fn approx_eq(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-6
    }

    #[test]
    fn temperature_requires_scale_and_converts() {
        let t = Temperature::new(212.0, TemperatureScale::Fahrenheit);
        assert!(approx_eq(t.value_in(TemperatureScale::Celsius), 100.0));
        assert!(approx_eq(t.value_in(TemperatureScale::Kelvin), 373.15));
    }

    #[test]
    fn money_is_value_object_and_enforces_currency() {
        let usd = Currency::new(
            "USD",
            2,
            CurrencyType::Fiat {
                country: "US".into(),
            },
        );
        let a = Money::from_major(10, usd.clone()); // $10.00
        let b = Money::from_minor(250, usd.clone()); // $2.50
        let c = a.checked_add(&b).unwrap();
        assert_eq!(c.amount_major_trunc(), 12); // $12.xx (truncating)

        let btc = Currency::new(
            "BTC",
            8,
            CurrencyType::Crypto {
                chain: "bitcoin".into(),
            },
        );
        let sat = Money::from_minor(1000, btc);
        assert!(a.checked_add(&sat).is_err());

        // Value equality
        let a2 = Money::from_major(10, usd);
        assert_eq!(a, a2);
    }

    #[test]
    fn money_add_commutative_and_associative_same_currency() {
        let usd = Currency::new(
            "USD",
            2,
            CurrencyType::Fiat {
                country: "US".into(),
            },
        );
        let a = Money::from_minor(500, usd.clone()); // $5.00
        let b = Money::from_minor(250, usd.clone()); // $2.50
        let c = Money::from_minor(125, usd.clone()); // $1.25

        // Commutative: a + b == b + a
        assert_eq!(a.checked_add(&b).unwrap(), b.checked_add(&a).unwrap());

        // Associative: (a + b) + c == a + (b + c)
        let left = a.checked_add(&b).unwrap().checked_add(&c).unwrap();
        let right = a.checked_add(&b.checked_add(&c).unwrap()).unwrap();
        assert_eq!(left, right);

        // Result minor units should match sum
        assert_eq!(left.amount_minor(), 500 + 250 + 125);
        assert_eq!(left.currency(), &usd);
    }

    #[test]
    fn money_exponent_behavior_for_jpy_and_btc() {
        // JPY has 0 exponent (no minor units)
        let jpy = Currency::new(
            "JPY",
            0,
            CurrencyType::Fiat {
                country: "JP".into(),
            },
        );
        let y1 = Money::from_major(123, jpy.clone());
        assert_eq!(y1.amount_minor(), 123);
        assert_eq!(y1.amount_major_trunc(), 123);

        // BTC typical exponent 8 (satoshis)
        let btc = Currency::new(
            "BTC",
            8,
            CurrencyType::Crypto {
                chain: "bitcoin".into(),
            },
        );
        let s1 = Money::from_minor(50, btc.clone()); // 50 sats
        let s2 = Money::from_minor(50, btc.clone()); // 50 sats
        let s100 = s1.checked_add(&s2).unwrap();
        assert_eq!(s100.amount_minor(), 100);
        assert_eq!(s100.currency(), &btc);
    }

    #[test]
    fn money_negative_amounts_and_truncation() {
        let usd = Currency::new(
            "USD",
            2,
            CurrencyType::Fiat {
                country: "US".into(),
            },
        );
        let a = Money::from_minor(500, usd.clone()); // $5.00
        let b = Money::from_minor(750, usd.clone()); // $7.50
        let neg = a.checked_sub(&b).unwrap(); // -$2.50
        assert_eq!(neg.amount_minor(), -250);
        // Integer division truncates toward zero: -250 / 100 => -2
        assert_eq!(neg.amount_major_trunc(), -2);
    }

    #[test]
    fn money_serde_roundtrip() {
        let eur = Currency::new(
            "EUR",
            2,
            CurrencyType::Fiat {
                country: "EU".into(),
            },
        );
        let m = Money::from_minor(12345, eur);
        let json = serde_json::to_string(&m).unwrap();
        let back: Money = serde_json::from_str(&json).unwrap();
        assert_eq!(m, back);
    }

    #[test]
    fn money_equality_depends_on_currency() {
        let pts_other = Currency::new(
            "PTS",
            2,
            CurrencyType::Other {
                description: "points".into(),
            },
        );
        let pts_chain = Currency::new(
            "PTS",
            2,
            CurrencyType::Crypto {
                chain: "sidechain".into(),
            },
        );
        let a = Money::from_minor(100, pts_other);
        let b = Money::from_minor(100, pts_chain);
        assert_ne!(a, b);
    }

    #[derive(Default)]
    struct StaticRates {
        entries: Vec<ConversionRate>,
    }
    impl RateProvider for StaticRates {
        fn latest_rate(&self, from: &Currency, to: &Currency) -> Option<(i128, i128)> {
            self.entries
                .iter()
                .filter(|r| &r.from == from && &r.to == to)
                .max_by_key(|r| r.at)
                .map(|r| (r.num, r.den))
        }
    }

    #[test]
    fn money_conversion_and_addition_in_target_currency() {
        // 1 GBP = 1.20 USD (both exp=2): num=120, den=100 for minor units
        let usd = Currency::new(
            "USD",
            2,
            CurrencyType::Fiat {
                country: "US".into(),
            },
        );
        let gbp = Currency::new(
            "GBP",
            2,
            CurrencyType::Fiat {
                country: "GB".into(),
            },
        );
        let mut rates = StaticRates::default();
        rates.entries.push(ConversionRate {
            from: gbp.clone(),
            to: usd.clone(),
            num: 120,
            den: 100,
            at: 1000,
        });

        let fifty_usd = Money::from_major(50, usd.clone());
        let fifty_gbp = Money::from_major(50, gbp.clone());

        let sum_usd = fifty_usd.add_in_currency(&fifty_gbp, &usd, &rates).unwrap();
        // 50 GBP -> 60 USD, + 50 USD => 110 USD (minor units: 11000)
        assert_eq!(sum_usd.currency(), &usd);
        assert_eq!(sum_usd.amount_minor(), 11000);
    }
}
