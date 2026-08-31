use actix_cors::Cors;
use actix_web::http::header;

/// Origins used when `CORS_ALLOWED_ORIGINS` is not set.
///
/// Deliberately a short localhost list rather than "allow anything": a wildcard
/// origin cannot be combined with credentialed requests, and silently allowing
/// every origin in production is worse than a loud CORS failure in development.
pub const DEFAULT_DEV_ORIGINS: &[&str] = &["http://localhost:5173", "http://127.0.0.1:5173"];

/// Parses the comma-separated `CORS_ALLOWED_ORIGINS` value.
///
/// Blank entries are dropped and surrounding whitespace trimmed, so
/// `"a, ,b,"` yields `["a", "b"]`. An input with no usable entries yields an
/// empty vec, which callers treat as "fall back to the dev defaults".
pub fn parse_allowed_origins(raw: Option<&str>) -> Vec<String> {
    raw.unwrap_or("")
        .split(',')
        .map(|origin| origin.trim())
        .filter(|origin| !origin.is_empty())
        .map(|origin| origin.to_string())
        .collect()
}

/// Builds the CORS middleware from `CORS_ALLOWED_ORIGINS`.
pub fn build_cors() -> Cors {
    let raw = std::env::var("CORS_ALLOWED_ORIGINS").ok();
    let configured = parse_allowed_origins(raw.as_deref());

    let mut cors = Cors::default()
        .allowed_methods(vec!["GET", "POST", "PUT", "PATCH", "DELETE", "OPTIONS"])
        .allowed_headers(vec![header::AUTHORIZATION, header::CONTENT_TYPE])
        .supports_credentials()
        .max_age(3600);

    if configured.is_empty() {
        tracing::warn!(
            "CORS_ALLOWED_ORIGINS not set; falling back to development origins: {:?}",
            DEFAULT_DEV_ORIGINS
        );
        for origin in DEFAULT_DEV_ORIGINS {
            cors = cors.allowed_origin(origin);
        }
    } else {
        for origin in &configured {
            cors = cors.allowed_origin(origin);
        }
    }

    cors
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_single_origin() {
        assert_eq!(
            parse_allowed_origins(Some("https://app.example.com")),
            vec!["https://app.example.com"]
        );
    }

    #[test]
    fn parses_multiple_origins_and_trims_whitespace() {
        assert_eq!(
            parse_allowed_origins(Some("https://a.com,  https://b.com ,https://c.com")),
            vec!["https://a.com", "https://b.com", "https://c.com"]
        );
    }

    #[test]
    fn drops_blank_entries_from_trailing_and_doubled_commas() {
        assert_eq!(
            parse_allowed_origins(Some("https://a.com,, ,https://b.com,")),
            vec!["https://a.com", "https://b.com"]
        );
    }

    #[test]
    fn returns_empty_for_missing_value() {
        assert!(parse_allowed_origins(None).is_empty());
    }

    #[test]
    fn returns_empty_for_value_with_no_usable_entries() {
        assert!(parse_allowed_origins(Some("  , ,")).is_empty());
    }
}
