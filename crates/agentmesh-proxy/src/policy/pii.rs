//! PII detection — regex-based scanner for sensitive data in agent payloads.
//!
//! Compliance-first invariant (compliance-first skill):
//! - NEVER store the actual PII value — only the type and location.
//! - All matches return `PiiMatch { pii_type, location }` with no raw text.
//!
//! Supported PII types:
//! - Email addresses
//! - US phone numbers (various formats)
//! - Social Security Numbers (XXX-XX-XXXX)
//! - Credit card numbers (basic Luhn-eligible 13–19 digit patterns)
//! - IPv4 addresses
//!
//! Note: The Rust `regex` crate does not support lookahead/lookbehind assertions.
//! SSN validation (excluding 000, 666, 9xx area codes and 00 group / 0000 serial)
//! is performed as a post-match validation step rather than in the regex itself.

#![allow(dead_code)]

use regex::Regex;
use serde::{Deserialize, Serialize};

// ── PII types ─────────────────────────────────────────────────────────────────

/// The category of personally-identifiable information detected.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PiiType {
    Email,
    Phone,
    Ssn,
    CreditCard,
    IpAddress,
}

impl std::fmt::Display for PiiType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PiiType::Email => write!(f, "EMAIL"),
            PiiType::Phone => write!(f, "PHONE"),
            PiiType::Ssn => write!(f, "SSN"),
            PiiType::CreditCard => write!(f, "CREDIT_CARD"),
            PiiType::IpAddress => write!(f, "IP_ADDRESS"),
        }
    }
}

// ── Match ─────────────────────────────────────────────────────────────────────

/// A PII detection result — type and location only, NEVER the actual value.
///
/// Per the compliance-first skill: "No PII values in the compliance envelope —
/// only types and locations."
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PiiMatch {
    /// What kind of PII was found.
    pub pii_type: PiiType,
    /// JSON-path or field name describing where in the data the PII was found.
    /// Example: `"messages[1].content"` or `"root"` for plain text scans.
    pub location: String,
}

// ── Internal pattern holder ────────────────────────────────────────────────────

struct PiiPattern {
    pii_type: PiiType,
    regex: Regex,
    /// Replacement label used in `redact_text` (e.g. `"[REDACTED:EMAIL]"`).
    redact_label: &'static str,
    /// Optional post-match validator.
    ///
    /// Because the Rust `regex` crate does not support lookahead/lookbehind,
    /// patterns that need exclusion rules (e.g. SSN area-code validation)
    /// use a plain regex for matching and this closure to reject false positives.
    validator: Option<fn(&str) -> bool>,
}

// ── Detector ──────────────────────────────────────────────────────────────────

/// Regex-based PII detector.
///
/// Patterns are compiled once at construction time and reused across all scans.
/// Construction is deliberately cheap after compilation (stack-allocated Vec).
///
/// Usage:
/// ```rust,no_run
/// use agentmesh_proxy::policy::pii::PiiDetector;
/// let detector = PiiDetector::new();
/// let text = "Contact me at alice@example.com or call 555-867-5309";
/// let matches = detector.scan_text(text);
/// assert_eq!(matches.len(), 2);
/// let redacted = detector.redact_text(text);
/// assert!(!redacted.contains("alice@example.com"));
/// ```
pub struct PiiDetector {
    patterns: Vec<PiiPattern>,
}

impl PiiDetector {
    /// Compile all PII detection regexes.
    ///
    /// Panics on startup if any regex is invalid (should never happen with
    /// the hard-coded patterns below — treated as a programming error).
    pub fn new() -> Self {
        let patterns = vec![
            // ── Email ──────────────────────────────────────────────────────────
            // RFC 5322 simplified — local@domain.tld
            PiiPattern {
                pii_type: PiiType::Email,
                regex: Regex::new(
                    r"(?i)[a-zA-Z0-9._%+\-]+@[a-zA-Z0-9.\-]+\.[a-zA-Z]{2,}"
                )
                .expect("email regex"),
                redact_label: "[REDACTED:EMAIL]",
                validator: None,
            },

            // ── US Phone numbers ───────────────────────────────────────────────
            // Matches: (555) 867-5309, 555-867-5309, 555.867.5309, 5558675309, +15558675309
            PiiPattern {
                pii_type: PiiType::Phone,
                regex: Regex::new(
                    r"(?:\+?1[\s\-.]?)?\(?\d{3}\)?[\s\-.]?\d{3}[\s\-.]?\d{4}"
                )
                .expect("phone regex"),
                redact_label: "[REDACTED:PHONE]",
                validator: None,
            },

            // ── SSN ────────────────────────────────────────────────────────────
            // Format: XXX-XX-XXXX (dashes required to reduce false positives).
            // The Rust regex crate does not support lookaheads, so we use a
            // simple pattern and apply post-match validation via `validator`.
            PiiPattern {
                pii_type: PiiType::Ssn,
                regex: Regex::new(r"\b\d{3}-\d{2}-\d{4}\b").expect("ssn regex"),
                redact_label: "[REDACTED:SSN]",
                validator: Some(is_valid_ssn),
            },

            // ── Credit card ────────────────────────────────────────────────────
            // Matches known card number prefixes and lengths.
            PiiPattern {
                pii_type: PiiType::CreditCard,
                regex: Regex::new(
                    r"\b(?:4[0-9]{12}(?:[0-9]{3})?|[25][1-7][0-9]{14}|6(?:011|5[0-9][0-9])[0-9]{12}|3[47][0-9]{13}|3(?:0[0-5]|[68][0-9])[0-9]{11}|(?:2131|1800|35\d{3})\d{11})\b"
                )
                .expect("credit card regex"),
                redact_label: "[REDACTED:CREDIT_CARD]",
                validator: None,
            },

            // ── IPv4 address ───────────────────────────────────────────────────
            // Matches dotted-decimal IPs; ignores obviously invalid octets (>255)
            PiiPattern {
                pii_type: PiiType::IpAddress,
                regex: Regex::new(
                    r"\b(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\b"
                )
                .expect("ip address regex"),
                redact_label: "[REDACTED:IP_ADDRESS]",
                validator: None,
            },
        ];

        Self { patterns }
    }

    /// Scan a plain-text string for PII matches.
    ///
    /// Returns a `Vec<PiiMatch>` with one entry per PII found.
    /// The match location is set to the `context_path` parameter so callers
    /// can provide the JSON field path (e.g. `"messages[0].content"`).
    pub fn scan_text(&self, text: &str) -> Vec<PiiMatch> {
        self.scan_text_with_path(text, "text")
    }

    /// Scan text and record matches at the given `location` path.
    ///
    /// Internal helper — used by both `scan_text` and `scan_json`.
    fn scan_text_with_path(&self, text: &str, location: &str) -> Vec<PiiMatch> {
        let mut matches = Vec::new();

        for pattern in &self.patterns {
            // Find if any regex match passes the optional post-match validator
            let found = pattern
                .regex
                .find_iter(text)
                .any(|m| match pattern.validator {
                    Some(validate) => validate(m.as_str()),
                    None => true,
                });

            if found {
                matches.push(PiiMatch {
                    pii_type: pattern.pii_type.clone(),
                    location: location.to_string(),
                    // NEVER store the matched text here — compliance rule
                });
            }
        }

        matches
    }

    /// Recursively scan a JSON value for PII in all string fields.
    ///
    /// Traverses the entire JSON tree and records the JSON-path location of
    /// every string field that contains PII. Array indices and object keys are
    /// included in the path (e.g. `"messages[1].content"`).
    ///
    /// # Arguments
    /// * `value` — the JSON value to scan
    /// * `path`  — the JSON path prefix for this node (use `""` for the root)
    pub fn scan_json(&self, value: &serde_json::Value, path: &str) -> Vec<PiiMatch> {
        let mut matches = Vec::new();
        self.scan_json_recursive(value, path, &mut matches);
        matches
    }

    fn scan_json_recursive(
        &self,
        value: &serde_json::Value,
        path: &str,
        matches: &mut Vec<PiiMatch>,
    ) {
        match value {
            serde_json::Value::String(s) => {
                let location = if path.is_empty() {
                    "root".to_string()
                } else {
                    path.to_string()
                };
                let found = self.scan_text_with_path(s, &location);
                matches.extend(found);
            }
            serde_json::Value::Object(map) => {
                for (key, val) in map {
                    let child_path = if path.is_empty() {
                        key.clone()
                    } else {
                        format!("{}.{}", path, key)
                    };
                    self.scan_json_recursive(val, &child_path, matches);
                }
            }
            serde_json::Value::Array(arr) => {
                for (i, val) in arr.iter().enumerate() {
                    let child_path = format!("{}[{}]", path, i);
                    self.scan_json_recursive(val, &child_path, matches);
                }
            }
            // Numbers, bools, null — not PII candidates
            _ => {}
        }
    }

    /// Replace all PII in `text` with `[REDACTED:TYPE]` placeholders.
    ///
    /// Applies all patterns in order. Multiple PII types can be redacted in
    /// a single pass (the replacements are applied sequentially, not in one pass,
    /// so overlapping matches use the last applied pattern).
    ///
    /// This is used for log-safe output. The STORED data MUST still use
    /// `PiiMatch` without values — this function is for display/reporting only.
    pub fn redact_text(&self, text: &str) -> String {
        let mut result = text.to_string();
        for pattern in &self.patterns {
            match pattern.validator {
                None => {
                    // No post-match validation — replace all matches unconditionally
                    result = pattern
                        .regex
                        .replace_all(&result, pattern.redact_label)
                        .to_string();
                }
                Some(validate) => {
                    // Replace only matches that pass the validator.
                    // We use replace_all with a closure that returns the original
                    // text for invalid matches, or the redact label for valid ones.
                    let label = pattern.redact_label;
                    result = pattern
                        .regex
                        .replace_all(&result, |caps: &regex::Captures| {
                            let matched = caps.get(0).map_or("", |m| m.as_str());
                            if validate(matched) {
                                label.to_string()
                            } else {
                                matched.to_string()
                            }
                        })
                        .to_string();
                }
            }
        }
        result
    }
}

impl Default for PiiDetector {
    fn default() -> Self {
        Self::new()
    }
}

// ── Post-match validators ─────────────────────────────────────────────────────

/// Validate that a matched SSN string is a real SSN (not an invalid test number).
///
/// SSN rules (per Social Security Administration):
/// - Area (first 3 digits): must not be 000, 666, or 900-999
/// - Group (digits 5-6): must not be 00
/// - Serial (last 4 digits): must not be 0000
///
/// The Rust `regex` crate does not support lookahead assertions, so these
/// exclusions are applied here as a post-match validation step.
fn is_valid_ssn(matched: &str) -> bool {
    // Expected format: "NNN-NN-NNNN"
    let parts: Vec<&str> = matched.split('-').collect();
    if parts.len() != 3 {
        return false;
    }

    let area = parts[0];
    let group = parts[1];
    let serial = parts[2];

    // Area must not be 000
    if area == "000" {
        return false;
    }
    // Area must not be 666
    if area == "666" {
        return false;
    }
    // Area must not be 900-999
    if let Ok(area_num) = area.parse::<u32>() {
        if area_num >= 900 {
            return false;
        }
    }
    // Group must not be 00
    if group == "00" {
        return false;
    }
    // Serial must not be 0000
    if serial == "0000" {
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn detector() -> PiiDetector {
        PiiDetector::new()
    }

    // ── Email ─────────────────────────────────────────────────────────────────

    #[test]
    fn detect_simple_email() {
        let m = detector().scan_text("Contact alice@example.com for info");
        assert!(m.iter().any(|m| m.pii_type == PiiType::Email));
    }

    #[test]
    fn detect_email_plus_addressing() {
        let m = detector().scan_text("Send to alice+test@sub.domain.org");
        assert!(m.iter().any(|m| m.pii_type == PiiType::Email));
    }

    #[test]
    fn no_email_in_clean_text() {
        let m = detector().scan_text("No personal data here, just plain text.");
        assert!(!m.iter().any(|m| m.pii_type == PiiType::Email));
    }

    // ── Phone ─────────────────────────────────────────────────────────────────

    #[test]
    fn detect_us_phone_dashes() {
        let m = detector().scan_text("Call me at 555-867-5309");
        assert!(m.iter().any(|m| m.pii_type == PiiType::Phone));
    }

    #[test]
    fn detect_us_phone_parens() {
        let m = detector().scan_text("My number is (555) 867-5309");
        assert!(m.iter().any(|m| m.pii_type == PiiType::Phone));
    }

    #[test]
    fn detect_us_phone_dotted() {
        let m = detector().scan_text("Reach me at 555.867.5309 anytime");
        assert!(m.iter().any(|m| m.pii_type == PiiType::Phone));
    }

    #[test]
    fn detect_us_phone_country_code() {
        let m = detector().scan_text("+1 (555) 867-5309");
        assert!(m.iter().any(|m| m.pii_type == PiiType::Phone));
    }

    // ── SSN ───────────────────────────────────────────────────────────────────

    #[test]
    fn detect_ssn_format() {
        let m = detector().scan_text("SSN: 123-45-6789");
        assert!(m.iter().any(|m| m.pii_type == PiiType::Ssn));
    }

    #[test]
    fn no_ssn_all_zeros() {
        // 000-XX-XXXX is invalid per SSN rules
        let m = detector().scan_text("fake: 000-45-6789");
        assert!(!m.iter().any(|m| m.pii_type == PiiType::Ssn));
    }

    #[test]
    fn no_ssn_group_666() {
        // 666-XX-XXXX is invalid
        let m = detector().scan_text("fake: 666-45-6789");
        assert!(!m.iter().any(|m| m.pii_type == PiiType::Ssn));
    }

    // ── Credit card ───────────────────────────────────────────────────────────

    #[test]
    fn detect_visa_card() {
        // Luhn-valid Visa test number
        let m = detector().scan_text("Card: 4111111111111111");
        assert!(m.iter().any(|m| m.pii_type == PiiType::CreditCard));
    }

    #[test]
    fn detect_mastercard() {
        // Luhn-valid Mastercard test number
        let m = detector().scan_text("Payment: 5500005555555559");
        assert!(m.iter().any(|m| m.pii_type == PiiType::CreditCard));
    }

    #[test]
    fn detect_amex() {
        // Amex test number
        let m = detector().scan_text("Pay with 378282246310005");
        assert!(m.iter().any(|m| m.pii_type == PiiType::CreditCard));
    }

    // ── IP address ────────────────────────────────────────────────────────────

    #[test]
    fn detect_ipv4_address() {
        let m = detector().scan_text("Server at 192.168.1.100");
        assert!(m.iter().any(|m| m.pii_type == PiiType::IpAddress));
    }

    #[test]
    fn detect_public_ip() {
        let m = detector().scan_text("Origin: 203.0.113.42");
        assert!(m.iter().any(|m| m.pii_type == PiiType::IpAddress));
    }

    #[test]
    fn no_invalid_ip_octet() {
        // 999.x.x.x is not a valid IP
        let m = detector().scan_text("Invalid: 999.168.1.1");
        assert!(!m.iter().any(|m| m.pii_type == PiiType::IpAddress));
    }

    // ── JSON scanning ─────────────────────────────────────────────────────────

    #[test]
    fn scan_json_finds_email_in_nested_field() {
        let json = serde_json::json!({
            "messages": [
                {"role": "user", "content": "My email is bob@test.org please help"}
            ]
        });
        let matches = detector().scan_json(&json, "");
        assert!(matches
            .iter()
            .any(|m| m.pii_type == PiiType::Email && m.location == "messages[0].content"));
    }

    #[test]
    fn scan_json_multiple_pii_types() {
        let json = serde_json::json!({
            "text": "Email alice@example.com SSN 123-45-6789"
        });
        let matches = detector().scan_json(&json, "");
        let types: Vec<&PiiType> = matches.iter().map(|m| &m.pii_type).collect();
        assert!(types.contains(&&PiiType::Email));
        assert!(types.contains(&&PiiType::Ssn));
    }

    #[test]
    fn scan_json_clean_payload_returns_empty() {
        let json = serde_json::json!({
            "model": "gpt-4o",
            "messages": [{"role": "user", "content": "What is the capital of France?"}]
        });
        let matches = detector().scan_json(&json, "");
        assert!(matches.is_empty());
    }

    #[test]
    fn scan_json_path_includes_array_index() {
        let json = serde_json::json!({
            "items": ["clean", "also clean", "call 555-867-5309 now"]
        });
        let matches = detector().scan_json(&json, "");
        assert!(matches
            .iter()
            .any(|m| m.pii_type == PiiType::Phone && m.location == "items[2]"));
    }

    #[test]
    fn scan_json_deeply_nested() {
        let json = serde_json::json!({
            "a": { "b": { "c": "user 192.168.1.1 accessed system" } }
        });
        let matches = detector().scan_json(&json, "");
        assert!(matches
            .iter()
            .any(|m| m.pii_type == PiiType::IpAddress && m.location == "a.b.c"));
    }

    // ── Redaction ─────────────────────────────────────────────────────────────

    #[test]
    fn redact_email_from_text() {
        let text = "Contact alice@example.com for details";
        let redacted = detector().redact_text(text);
        assert!(!redacted.contains("alice@example.com"));
        assert!(redacted.contains("[REDACTED:EMAIL]"));
    }

    #[test]
    fn redact_phone_from_text() {
        let text = "Call 555-867-5309 now";
        let redacted = detector().redact_text(text);
        assert!(!redacted.contains("555-867-5309"));
        assert!(redacted.contains("[REDACTED:PHONE]"));
    }

    #[test]
    fn redact_ssn_from_text() {
        let text = "SSN is 123-45-6789";
        let redacted = detector().redact_text(text);
        assert!(!redacted.contains("123-45-6789"));
        assert!(redacted.contains("[REDACTED:SSN]"));
    }

    #[test]
    fn redact_preserves_non_pii_text() {
        let text = "The answer to life is 42";
        let redacted = detector().redact_text(text);
        assert_eq!(redacted, text);
    }

    #[test]
    fn redact_multiple_pii_types() {
        let text = "Email alice@example.com phone 555-867-5309";
        let redacted = detector().redact_text(text);
        assert!(!redacted.contains("alice@example.com"));
        assert!(!redacted.contains("555-867-5309"));
        assert!(redacted.contains("[REDACTED:EMAIL]"));
        assert!(redacted.contains("[REDACTED:PHONE]"));
    }

    // ── No stored values ──────────────────────────────────────────────────────

    #[test]
    fn pii_match_never_stores_actual_value() {
        // Compile-time verification: PiiMatch has no `value` or `text` field.
        // This test serves as a documentation assertion.
        let m = PiiMatch {
            pii_type: PiiType::Email,
            location: "test_field".to_string(),
            // `value` field does not exist — would fail to compile if added without notice
        };
        // Only type and location are accessible
        assert_eq!(m.pii_type, PiiType::Email);
        assert_eq!(m.location, "test_field");
    }
}
