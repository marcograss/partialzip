use url::Url;

/// Returns if a URL is a valid URL string supported by the library
pub fn url_is_valid(url: &str) -> bool {
    if let Ok(url) = Url::parse(url) {
        if url.scheme() == "http" || url.scheme() == "https" || url.scheme() == "ftp" {
            return true;
        }
    }
    false
}
