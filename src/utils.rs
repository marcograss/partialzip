use url::Url;

/// Returns if a URL is a valid URL string supported by the library
#[must_use]
pub fn url_is_valid(url: &str) -> bool {
    match Url::parse(url) {
        Ok(url) => ["http", "https", "ftp"].contains(&url.scheme()),
        Err(_) => false,
    }
}
