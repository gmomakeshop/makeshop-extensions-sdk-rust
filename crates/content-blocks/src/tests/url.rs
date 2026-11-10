//! URL hygiene checks for the rules in [`crate::url`].
//!
//! Destinations are not allowlisted. These checks only require an HTTPS URL that the
//! renderer can follow safely.

use crate::{Block, Card, ContentBlocksError, HttpsUrl, Image, Link, Qr, UrlError};

fn url_error(block: impl Into<Block>) -> UrlError {
    match block.into().validate().expect_err("must be rejected") {
        ContentBlocksError::Url {
            reason,
            ..
        } => reason,
        other => panic!("expected a url error, got {other}"),
    }
}

#[test]
fn rejects_non_https_schemes() {
    // Reject javascript: and data: URLs because they could expose an XSS path in the renderer.
    assert!(matches!(url_error(Link::new("x", "http://example.com/a")), UrlError::NotHttps { .. }));
    assert!(matches!(url_error(Link::new("x", "javascript:alert(1)")), UrlError::NotHttps { .. }));
    assert!(matches!(url_error(Link::new("x", "data:text/html,<script>")), UrlError::NotHttps { .. }));
}

#[test]
fn rejects_userinfo() {
    // Userinfo can disguise a destination and enable phishing.
    assert!(matches!(url_error(Link::new("x", "https://user:pass@evil.example.org/a")), UrlError::Userinfo { .. }));
}

#[test]
fn rejects_forbidden_chars() {
    assert!(matches!(url_error(Link::new("x", "https://example.com/a b")), UrlError::ForbiddenChars));
    assert!(matches!(url_error(Link::new("x", "https://example.com/a\\b")), UrlError::ForbiddenChars));
    assert!(matches!(url_error(Link::new("x", "https://example.com/a\nb")), UrlError::ForbiddenChars));
}

#[test]
fn rejects_html_attribute_breaking_chars() {
    // A renderer that forgets to escape an attribute value must not let a URL close it or open a tag.
    for url in [
        "https://example.com/\"onmouseover=\"alert(1)",
        "https://example.com/'onmouseover='alert(1)",
        "https://example.com/a<script>",
        "https://example.com/a>b",
        "https://example.com/`onmouseover=alert(1)",
        "https://example.com/?q=\"",
    ] {
        assert!(matches!(url_error(Link::new("x", url)), UrlError::ForbiddenChars), "{url}");
        assert!(matches!(url_error(Image::new(url, "alt")), UrlError::ForbiddenChars), "{url}");
    }
}

#[test]
fn accepts_query_strings_and_percent_encoded_quotes() {
    for url in ["https://example.com/search?q=a&page=2", "https://example.com/o%27reilly?title=%22x%22"] {
        Block::from(Link::new("x", url))
            .validate()
            .unwrap_or_else(|err| panic!("{url}: {err}"));
    }
}

#[test]
fn rejects_missing_host() {
    assert!(matches!(url_error(Link::new("x", "https://:8080/a")), UrlError::MissingHost { .. }));
}

#[test]
fn accepts_any_https_host() {
    // Connector declarations do not restrict destinations; there is no host allowlist.
    for url in ["https://example.com/promo", "https://EXAMPLE.com/a", "https://cdn.example.net:8443/a.png?v=1#f"] {
        Block::from(Link::new("詳細", url))
            .validate()
            .unwrap_or_else(|err| panic!("{url}: {err}"));
    }
}

#[test]
fn card_image_and_link_urls_are_checked_too() {
    assert!(matches!(
        url_error(Card::new("t").image(Image::new("http://cdn.example.com/c.png", "alt"))),
        UrlError::NotHttps { .. }
    ));
    assert!(matches!(
        url_error(Card::new("t").link(Link::new("go", "javascript:alert(1)"))),
        UrlError::NotHttps { .. }
    ));
}

#[test]
fn qr_value_is_not_url_checked() {
    // The renderer encodes this value as a QR image and never follows it, so any scheme is valid.
    Block::from(Qr::new("https://example.org/x"))
        .validate()
        .expect("https value");
    Block::from(Qr::new("MEMBER-0001"))
        .validate()
        .expect("non-url value");
}

#[test]
fn error_message_names_the_field() {
    let err = Block::from(Image::new("http://cdn.example.com/a.png", "alt"))
        .validate()
        .expect_err("http");
    assert_eq!(err.to_string(), "image.url must be an https:// URL (got 'http://cdn.example.com/a.png')");

    let err = Block::from(Card::new("t").link(Link::new("go", "http://example.com/x")))
        .validate()
        .expect_err("http");
    assert!(err.to_string().starts_with("card.link.url must be"), "{err}");
}

#[test]
fn https_url_exposes_authority() {
    let url = HttpsUrl::parse("https://Example.com:8443/path?q=1#f").expect("parse");
    assert_eq!(url.authority(), "Example.com:8443");
    assert_eq!(url.as_str(), "https://Example.com:8443/path?q=1#f");
}
