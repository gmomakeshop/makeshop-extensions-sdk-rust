//! Validation of URLs used by links and images.
//!
//! Validation covers the HTTPS scheme, forbidden characters, user information, and host presence.
//! It does not enforce a destination allowlist.

use crate::error::{ContentBlocksError, UrlError};

const HTTPS_PREFIX: &str = "https://";

/// An HTTPS URL validated for use in Content Blocks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HttpsUrl<'a> {
    raw: &'a str,
    authority: &'a str,
}

impl<'a> HttpsUrl<'a> {
    /// Validates a URL.
    pub fn parse(raw: &'a str) -> Result<HttpsUrl<'a>, UrlError> {
        let Some(after_scheme) = raw.strip_prefix(HTTPS_PREFIX) else {
            return Err(UrlError::NotHttps {
                value: raw.to_string(),
            });
        };
        if raw.chars().any(is_forbidden_char) {
            return Err(UrlError::ForbiddenChars);
        }

        // The authority runs from immediately after the scheme to the first '/', '?', or '#'.
        let authority_end = after_scheme
            .find(['/', '?', '#'])
            .unwrap_or(after_scheme.len());
        let authority = &after_scheme[..authority_end];

        if authority.contains('@') {
            return Err(UrlError::Userinfo {
                value: raw.to_string(),
            });
        }
        // Reject URLs with an empty host, such as `https://:8080`.
        if authority.split(':').next().unwrap_or("").is_empty() {
            return Err(UrlError::MissingHost {
                value: raw.to_string(),
            });
        }

        Ok(HttpsUrl {
            raw,
            authority,
        })
    }

    /// Returns the URL provided during validation.
    pub fn as_str(&self) -> &'a str {
        self.raw
    }

    /// Returns the URL authority.
    pub fn authority(&self) -> &'a str {
        self.authority
    }
}

/// Reports whether `c` may not appear in a URL.
///
/// Besides control characters, whitespace, and backslashes, this rejects characters that can
/// end or open an HTML attribute value or tag: `"`, `'`, `<`, `>`, and `` ` ``. Percent-encode
/// them when a destination needs them. `&` remains valid so that query strings work.
fn is_forbidden_char(c: char) -> bool {
    c.is_control() || c.is_whitespace() || matches!(c, '\\' | '"' | '\'' | '<' | '>' | '`')
}

/// Validates a URL and attaches the field name to any error.
pub(crate) fn check_url(field: &'static str, url: &str) -> Result<(), ContentBlocksError> {
    HttpsUrl::parse(url).map_err(|reason| ContentBlocksError::Url {
        field,
        reason,
    })?;
    Ok(())
}
