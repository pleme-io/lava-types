//! lava-types — typed constraint validators for lava architectures.
//!
//! Pangea Dry::Struct analog. Every architecture input or resource
//! attribute can carry a typed constraint that runs at composition
//! time, NOT apply time. Invalid CIDR fails the plan; never reaches
//! the cloud API.
//!
//! ## Tatara-lisp surface
//!
//! ```lisp
//! (deflava-architecture aws-vpc-network
//!   :inputs ((:cidr  :type :cidr-block :default "10.0.0.0/16")
//!            (:azs   :type (:list-of :availability-zone) :min-items 1)
//!            (:env   :type (:enum "prod" "staging" "dev"))
//!            (:port  :type (:port-range 1024 65535) :default 8080)
//!            (:host  :type :hostname)
//!            (:extra :type :any))  ;; loose escape hatch
//!   :resources ...)
//! ```
//!
//! ## Strict + loose by design
//!
//! Strict: 14 typed primitives validate at compile time
//! (CIDR/port/regex/enum/length/range/IPv4/IPv6/hostname/URL/email/etc).
//!
//! Loose: `:any` / `:dynamic` accept any value. Use for extension
//! slots where the substrate intentionally hands off to a downstream
//! consumer.

#![allow(clippy::module_name_repetitions)]

use serde::{Deserialize, Serialize};
use std::net::{Ipv4Addr, Ipv6Addr};
use thiserror::Error;

/// A typed constraint. Each variant validates a string value against
/// a specific shape. `validate(&str)` is the single chokepoint
/// consumers call.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum Type {
    /// Any string. Loose escape hatch.
    Any,
    /// Dynamic (runtime-typed) — same as Any but signals intent.
    Dynamic,
    /// Bare string.
    String,
    /// Bare integer (parseable as i64).
    Integer,
    /// Bare boolean.
    Boolean,
    /// IPv4 + CIDR suffix (`10.0.0.0/16`).
    CidrBlock,
    /// Bare IPv4 (`10.0.0.1`).
    Ipv4,
    /// Bare IPv6 (`2001:db8::1`).
    Ipv6,
    /// Hostname per RFC 1123 (labels + dots, 1-253 chars).
    Hostname,
    /// Port number in `[lo, hi]`. Use [`Type::port_range`] for the
    /// most common case (1024-65535).
    PortRange { lo: u16, hi: u16 },
    /// String matching one of a fixed value set.
    Enum { values: Vec<String> },
    /// Integer in `[lo, hi]`.
    IntRange { lo: i64, hi: i64 },
    /// String length in `[min, max]` chars.
    Length { min: usize, max: usize },
    /// Regular expression. Stored as the source string; matched via
    /// a minimal substring contains-check (lava-types ships zero
    /// external deps — full regex moves in via `regex` feature later).
    /// For now: `match_kind` controls the semantics.
    Pattern { source: String, match_kind: MatchKind },
    /// List of inner types. Optional `min_items` / `max_items`
    /// validate cardinality.
    ListOf {
        inner: Box<Type>,
        min_items: Option<usize>,
        max_items: Option<usize>,
    },
    /// AWS availability zone (`us-east-1a` shape: region + single letter).
    AvailabilityZone,
    /// Email — RFC 5322 minimal shape (`localpart@domain`).
    Email,
    /// URL — `<scheme>://<rest>` shape; scheme must be present.
    Url,
}

/// Match semantics for [`Type::Pattern`].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MatchKind {
    /// Source substring must appear in input.
    Contains,
    /// Input must start with source.
    StartsWith,
    /// Input must end with source.
    EndsWith,
}

impl Type {
    /// Common: `:port-range 1024 65535`.
    #[must_use]
    pub fn port_range() -> Self {
        Self::PortRange {
            lo: 1024,
            hi: 65535,
        }
    }

    /// Validate a string input against this type.
    pub fn validate(&self, value: &str) -> Result<(), TypeError> {
        match self {
            Self::Any | Self::Dynamic | Self::String => Ok(()),
            Self::Integer => {
                value
                    .parse::<i64>()
                    .map(|_| ())
                    .map_err(|_| TypeError::NotInteger(value.to_string()))
            }
            Self::Boolean => match value {
                "true" | "false" | "#t" | "#f" => Ok(()),
                other => Err(TypeError::NotBoolean(other.to_string())),
            },
            Self::CidrBlock => validate_cidr_block(value),
            Self::Ipv4 => value
                .parse::<Ipv4Addr>()
                .map(|_| ())
                .map_err(|_| TypeError::NotIpv4(value.to_string())),
            Self::Ipv6 => value
                .parse::<Ipv6Addr>()
                .map(|_| ())
                .map_err(|_| TypeError::NotIpv6(value.to_string())),
            Self::Hostname => validate_hostname(value),
            Self::PortRange { lo, hi } => {
                let n: u16 = value
                    .parse()
                    .map_err(|_| TypeError::NotInteger(value.to_string()))?;
                if n < *lo || n > *hi {
                    return Err(TypeError::OutOfRange {
                        value: i64::from(n),
                        lo: i64::from(*lo),
                        hi: i64::from(*hi),
                    });
                }
                Ok(())
            }
            Self::Enum { values } => {
                if values.iter().any(|v| v == value) {
                    Ok(())
                } else {
                    Err(TypeError::NotInEnum {
                        value: value.to_string(),
                        choices: values.clone(),
                    })
                }
            }
            Self::IntRange { lo, hi } => {
                let n: i64 = value
                    .parse()
                    .map_err(|_| TypeError::NotInteger(value.to_string()))?;
                if n < *lo || n > *hi {
                    return Err(TypeError::OutOfRange { value: n, lo: *lo, hi: *hi });
                }
                Ok(())
            }
            Self::Length { min, max } => {
                let n = value.chars().count();
                if n < *min || n > *max {
                    return Err(TypeError::BadLength { len: n, min: *min, max: *max });
                }
                Ok(())
            }
            Self::Pattern { source, match_kind } => match match_kind {
                MatchKind::Contains => {
                    if value.contains(source.as_str()) {
                        Ok(())
                    } else {
                        Err(TypeError::PatternMismatch {
                            pattern: source.clone(),
                            value: value.to_string(),
                        })
                    }
                }
                MatchKind::StartsWith => {
                    if value.starts_with(source.as_str()) {
                        Ok(())
                    } else {
                        Err(TypeError::PatternMismatch {
                            pattern: source.clone(),
                            value: value.to_string(),
                        })
                    }
                }
                MatchKind::EndsWith => {
                    if value.ends_with(source.as_str()) {
                        Ok(())
                    } else {
                        Err(TypeError::PatternMismatch {
                            pattern: source.clone(),
                            value: value.to_string(),
                        })
                    }
                }
            },
            Self::ListOf { .. } => {
                // ListOf validates a list — not a scalar. Caller
                // should use validate_list for typed iteration.
                Err(TypeError::WrongValidator("list type — use validate_list"))
            }
            Self::AvailabilityZone => validate_availability_zone(value),
            Self::Email => validate_email(value),
            Self::Url => validate_url(value),
        }
    }

    /// Validate a list of string inputs. Honors `min_items` / `max_items`
    /// if the type is [`Type::ListOf`]; otherwise errors.
    pub fn validate_list(&self, values: &[&str]) -> Result<(), TypeError> {
        let Self::ListOf {
            inner,
            min_items,
            max_items,
        } = self
        else {
            return Err(TypeError::WrongValidator("expected ListOf"));
        };
        if let Some(min) = min_items {
            if values.len() < *min {
                return Err(TypeError::TooFewItems {
                    count: values.len(),
                    min: *min,
                });
            }
        }
        if let Some(max) = max_items {
            if values.len() > *max {
                return Err(TypeError::TooManyItems {
                    count: values.len(),
                    max: *max,
                });
            }
        }
        for v in values {
            inner.validate(v)?;
        }
        Ok(())
    }
}

#[derive(Debug, Error, PartialEq)]
pub enum TypeError {
    #[error("`{0}` is not a valid integer")]
    NotInteger(String),
    #[error("`{0}` is not a valid boolean (expected true|false|#t|#f)")]
    NotBoolean(String),
    #[error("`{0}` is not a valid IPv4 CIDR block (expected like 10.0.0.0/16)")]
    NotCidr(String),
    #[error("`{0}` is not a valid IPv4 address")]
    NotIpv4(String),
    #[error("`{0}` is not a valid IPv6 address")]
    NotIpv6(String),
    #[error("`{0}` is not a valid RFC 1123 hostname")]
    NotHostname(String),
    #[error("`{0}` is not a valid AWS availability zone (expected like us-east-1a)")]
    NotAvailabilityZone(String),
    #[error("`{0}` is not a valid email (expected localpart@domain)")]
    NotEmail(String),
    #[error("`{0}` is not a valid URL (expected scheme://...)")]
    NotUrl(String),
    #[error("value {value} out of range [{lo}, {hi}]")]
    OutOfRange { value: i64, lo: i64, hi: i64 },
    #[error("`{value}` not in enum [{}]", choices.join(", "))]
    NotInEnum { value: String, choices: Vec<String> },
    #[error("length {len} out of range [{min}, {max}]")]
    BadLength { len: usize, min: usize, max: usize },
    #[error("`{value}` doesn't match pattern `{pattern}`")]
    PatternMismatch { pattern: String, value: String },
    #[error("list has too few items: {count} < {min}")]
    TooFewItems { count: usize, min: usize },
    #[error("list has too many items: {count} > {max}")]
    TooManyItems { count: usize, max: usize },
    #[error("wrong validator: {0}")]
    WrongValidator(&'static str),
}

// ── Per-type validators (no external deps) ────────────────────────────

fn validate_cidr_block(value: &str) -> Result<(), TypeError> {
    let (ip, prefix) = value
        .split_once('/')
        .ok_or_else(|| TypeError::NotCidr(value.to_string()))?;
    ip.parse::<Ipv4Addr>()
        .map_err(|_| TypeError::NotCidr(value.to_string()))?;
    let p: u8 = prefix
        .parse()
        .map_err(|_| TypeError::NotCidr(value.to_string()))?;
    if p > 32 {
        return Err(TypeError::NotCidr(value.to_string()));
    }
    Ok(())
}

fn validate_hostname(value: &str) -> Result<(), TypeError> {
    if value.is_empty() || value.len() > 253 {
        return Err(TypeError::NotHostname(value.to_string()));
    }
    for label in value.split('.') {
        if label.is_empty() || label.len() > 63 {
            return Err(TypeError::NotHostname(value.to_string()));
        }
        if label.starts_with('-') || label.ends_with('-') {
            return Err(TypeError::NotHostname(value.to_string()));
        }
        if !label
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-')
        {
            return Err(TypeError::NotHostname(value.to_string()));
        }
    }
    Ok(())
}

fn validate_availability_zone(value: &str) -> Result<(), TypeError> {
    // Region (3+ chars with at least one digit) + single trailing letter.
    let last = value
        .chars()
        .last()
        .ok_or_else(|| TypeError::NotAvailabilityZone(value.to_string()))?;
    if !last.is_ascii_alphabetic() {
        return Err(TypeError::NotAvailabilityZone(value.to_string()));
    }
    let region = &value[..value.len() - 1];
    if region.len() < 3 || !region.contains(|c: char| c.is_ascii_digit()) {
        return Err(TypeError::NotAvailabilityZone(value.to_string()));
    }
    Ok(())
}

fn validate_email(value: &str) -> Result<(), TypeError> {
    let (local, domain) = value
        .split_once('@')
        .ok_or_else(|| TypeError::NotEmail(value.to_string()))?;
    if local.is_empty() || domain.is_empty() {
        return Err(TypeError::NotEmail(value.to_string()));
    }
    validate_hostname(domain).map_err(|_| TypeError::NotEmail(value.to_string()))?;
    Ok(())
}

fn validate_url(value: &str) -> Result<(), TypeError> {
    let (scheme, rest) = value
        .split_once("://")
        .ok_or_else(|| TypeError::NotUrl(value.to_string()))?;
    if scheme.is_empty() || rest.is_empty() {
        return Err(TypeError::NotUrl(value.to_string()));
    }
    if !scheme.chars().all(|c| c.is_ascii_alphanumeric() || c == '+') {
        return Err(TypeError::NotUrl(value.to_string()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn any_accepts_everything() {
        assert!(Type::Any.validate("").is_ok());
        assert!(Type::Any.validate("anything").is_ok());
    }

    #[test]
    fn cidr_block_validates_correctly() {
        assert!(Type::CidrBlock.validate("10.0.0.0/16").is_ok());
        assert!(Type::CidrBlock.validate("172.16.0.0/12").is_ok());
        assert!(Type::CidrBlock.validate("0.0.0.0/0").is_ok());
        assert!(Type::CidrBlock.validate("10.0.0.0/33").is_err()); // prefix > 32
        assert!(Type::CidrBlock.validate("10.0.0.0").is_err()); // no /
        assert!(Type::CidrBlock.validate("not-an-ip/16").is_err());
    }

    #[test]
    fn port_range_default_validates() {
        let t = Type::port_range();
        assert!(t.validate("8080").is_ok());
        assert!(t.validate("1024").is_ok());
        assert!(t.validate("65535").is_ok());
        assert!(t.validate("80").is_err()); // < 1024
        assert!(t.validate("65536").is_err()); // > 65535
        assert!(t.validate("abc").is_err());
    }

    #[test]
    fn enum_validates_membership() {
        let t = Type::Enum { values: vec!["prod".into(), "staging".into(), "dev".into()] };
        assert!(t.validate("prod").is_ok());
        assert!(t.validate("dev").is_ok());
        let err = t.validate("preview").unwrap_err();
        assert!(matches!(err, TypeError::NotInEnum { .. }));
    }

    #[test]
    fn availability_zone_validates_aws_shape() {
        assert!(Type::AvailabilityZone.validate("us-east-1a").is_ok());
        assert!(Type::AvailabilityZone.validate("eu-west-2b").is_ok());
        assert!(Type::AvailabilityZone.validate("ap-southeast-3c").is_ok());
        assert!(Type::AvailabilityZone.validate("us-east").is_err()); // no trailing letter
        assert!(Type::AvailabilityZone.validate("xx").is_err());
    }

    #[test]
    fn hostname_validates_rfc1123() {
        assert!(Type::Hostname.validate("example.com").is_ok());
        assert!(Type::Hostname.validate("sub.example.co.uk").is_ok());
        assert!(Type::Hostname.validate("-leading-dash.com").is_err());
        assert!(Type::Hostname.validate("trailing-dash-.com").is_err());
        assert!(Type::Hostname.validate("space invalid.com").is_err());
        assert!(Type::Hostname.validate("").is_err());
    }

    #[test]
    fn email_validates_minimal() {
        assert!(Type::Email.validate("a@example.com").is_ok());
        assert!(Type::Email.validate("noatsign").is_err());
        assert!(Type::Email.validate("@nolocal").is_err());
        assert!(Type::Email.validate("nodomain@").is_err());
    }

    #[test]
    fn url_validates_scheme_present() {
        assert!(Type::Url.validate("https://example.com").is_ok());
        assert!(Type::Url.validate("s3://bucket/key").is_ok());
        assert!(Type::Url.validate("git+ssh://x").is_ok());
        assert!(Type::Url.validate("noscheme.com").is_err());
        assert!(Type::Url.validate("://nopath").is_err());
    }

    #[test]
    fn pattern_match_kinds() {
        let starts = Type::Pattern {
            source: "https://".into(),
            match_kind: MatchKind::StartsWith,
        };
        assert!(starts.validate("https://x").is_ok());
        assert!(starts.validate("http://x").is_err());

        let ends = Type::Pattern {
            source: ".tlisp".into(),
            match_kind: MatchKind::EndsWith,
        };
        assert!(ends.validate("foo.tlisp").is_ok());
        assert!(ends.validate("foo.rb").is_err());

        let contains = Type::Pattern {
            source: "pleme".into(),
            match_kind: MatchKind::Contains,
        };
        assert!(contains.validate("pleme-io/lava").is_ok());
        assert!(contains.validate("nothing").is_err());
    }

    #[test]
    fn list_of_validates_cardinality_and_items() {
        let t = Type::ListOf {
            inner: Box::new(Type::AvailabilityZone),
            min_items: Some(1),
            max_items: Some(3),
        };
        assert!(t.validate_list(&["us-east-1a"]).is_ok());
        assert!(t.validate_list(&["us-east-1a", "us-east-1b"]).is_ok());
        let err = t.validate_list(&[]).unwrap_err();
        assert!(matches!(err, TypeError::TooFewItems { .. }));
        let err = t.validate_list(&["a", "b", "c", "d"]).unwrap_err();
        assert!(matches!(err, TypeError::TooManyItems { .. }));
        // Item-level validation: invalid AZ fails.
        let err = t.validate_list(&["us-east-1a", "not-a-zone"]).unwrap_err();
        assert!(matches!(err, TypeError::NotAvailabilityZone(_)));
    }

    #[test]
    fn length_validates_char_count() {
        let t = Type::Length { min: 3, max: 10 };
        assert!(t.validate("abc").is_ok());
        assert!(t.validate("abcdefghij").is_ok());
        assert!(t.validate("ab").is_err());
        assert!(t.validate("abcdefghijk").is_err());
    }

    #[test]
    fn type_round_trips_through_serde() {
        // Critical: every typed validator must serialize/deserialize so
        // .tlisp parsed types can be persisted in caixa sources.
        let t = Type::ListOf {
            inner: Box::new(Type::AvailabilityZone),
            min_items: Some(1),
            max_items: Some(5),
        };
        let json = serde_json::to_string(&t).unwrap();
        let parsed: Type = serde_json::from_str(&json).unwrap();
        assert_eq!(t, parsed);
    }
}
