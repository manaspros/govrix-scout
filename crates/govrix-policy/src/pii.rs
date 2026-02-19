use regex::Regex;
use std::sync::OnceLock;

struct PiiPatterns {
    email: Regex,
    phone: Regex,
    ssn: Regex,
    credit_card: Regex,
    ip: Regex,
}

static PII_PATTERNS: OnceLock<PiiPatterns> = OnceLock::new();

fn patterns() -> &'static PiiPatterns {
    PII_PATTERNS.get_or_init(|| PiiPatterns {
        email: Regex::new(r"[a-zA-Z0-9._%+\-]+@[a-zA-Z0-9.\-]+\.[a-zA-Z]{2,}").unwrap(),
        phone: Regex::new(r"\b\d{3}[-.]?\d{3}[-.]?\d{4}\b").unwrap(),
        ssn: Regex::new(r"\b\d{3}-\d{2}-\d{4}\b").unwrap(),
        credit_card: Regex::new(r"\b\d{4}[-\s]?\d{4}[-\s]?\d{4}[-\s]?\d{4}\b").unwrap(),
        ip: Regex::new(r"\b\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}\b").unwrap(),
    })
}

/// Replace PII found in `text` with labelled tokens.
///
/// Patterns are applied in specificity order so that a more-specific pattern
/// (SSN, credit card) is applied before a broader one (phone, IP) that might
/// consume overlapping digit sequences.
///
/// | PII type    | Replacement     |
/// |-------------|-----------------|
/// | SSN         | `[SSN]`         |
/// | Credit card | `[CREDIT_CARD]` |
/// | Email       | `[EMAIL]`       |
/// | Phone       | `[PHONE]`       |
/// | IP address  | `[IP]`          |
pub fn mask_pii(text: &str) -> String {
    let p = patterns();

    let s = p.ssn.replace_all(text, "[SSN]");
    let s = p.credit_card.replace_all(&s, "[CREDIT_CARD]");
    let s = p.email.replace_all(&s, "[EMAIL]");
    let s = p.phone.replace_all(&s, "[PHONE]");
    let s = p.ip.replace_all(&s, "[IP]");

    s.into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Email ──────────────────────────────────────────────────────────────────

    #[test]
    fn masks_email() {
        assert_eq!(
            mask_pii("contact hello@example.com now"),
            "contact [EMAIL] now"
        );
    }

    #[test]
    fn masks_email_subdomain() {
        assert_eq!(
            mask_pii("reach support@mail.example.org please"),
            "reach [EMAIL] please"
        );
    }

    // ── Phone ──────────────────────────────────────────────────────────────────

    #[test]
    fn masks_phone_dashes() {
        assert_eq!(mask_pii("call 555-867-5309 today"), "call [PHONE] today");
    }

    #[test]
    fn masks_phone_dots() {
        assert_eq!(mask_pii("reach me at 555.867.5309"), "reach me at [PHONE]");
    }

    #[test]
    fn masks_phone_plain() {
        assert_eq!(mask_pii("number is 5558675309"), "number is [PHONE]");
    }

    // ── SSN ────────────────────────────────────────────────────────────────────

    #[test]
    fn masks_ssn() {
        assert_eq!(mask_pii("SSN: 123-45-6789"), "SSN: [SSN]");
    }

    #[test]
    fn masks_ssn_in_sentence() {
        let result = mask_pii("Employee SSN is 078-05-1120.");
        assert!(!result.contains("078-05-1120"));
        assert!(result.contains("[SSN]"));
    }

    // ── Credit card ────────────────────────────────────────────────────────────

    #[test]
    fn masks_credit_card_plain() {
        assert_eq!(mask_pii("card 4111111111111111"), "card [CREDIT_CARD]");
    }

    #[test]
    fn masks_credit_card_dashes() {
        assert_eq!(mask_pii("card 4111-1111-1111-1111"), "card [CREDIT_CARD]");
    }

    #[test]
    fn masks_credit_card_spaces() {
        assert_eq!(mask_pii("card 4111 1111 1111 1111"), "card [CREDIT_CARD]");
    }

    // ── IP address ─────────────────────────────────────────────────────────────

    #[test]
    fn masks_ip() {
        assert_eq!(
            mask_pii("origin 192.168.1.1 blocked"),
            "origin [IP] blocked"
        );
    }

    #[test]
    fn masks_ip_loopback() {
        assert_eq!(mask_pii("from 127.0.0.1"), "from [IP]");
    }

    // ── Mixed / edge cases ─────────────────────────────────────────────────────

    #[test]
    fn masks_mixed_pii() {
        let input = "email hello@test.com phone 555-123-4567 ssn 111-22-3333 ip 10.0.0.1";
        assert_eq!(
            mask_pii(input),
            "email [EMAIL] phone [PHONE] ssn [SSN] ip [IP]"
        );
    }

    #[test]
    fn masks_all_five_types() {
        let input = "Email: a@b.com Card: 4111-1111-1111-1111 SSN: 123-45-6789 Phone: 800-555-0199 IP: 1.2.3.4";
        let result = mask_pii(input);
        assert!(result.contains("[EMAIL]"), "email not masked: {result}");
        assert!(
            result.contains("[CREDIT_CARD]"),
            "card not masked: {result}"
        );
        assert!(result.contains("[SSN]"), "ssn not masked: {result}");
        assert!(result.contains("[PHONE]"), "phone not masked: {result}");
        assert!(result.contains("[IP]"), "ip not masked: {result}");
        assert!(!result.contains('@'), "raw email leaked: {result}");
    }

    #[test]
    fn no_pii_unchanged() {
        let clean = "The quick brown fox jumps over the lazy dog.";
        assert_eq!(mask_pii(clean), clean);
    }

    #[test]
    fn empty_string_unchanged() {
        assert_eq!(mask_pii(""), "");
    }
}
