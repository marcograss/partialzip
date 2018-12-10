use url::Url;

pub fn url_is_valid(url: &str) -> bool {
    if let Ok(url) = Url::parse(url) {
        if url.scheme() == "http" || url.scheme() == "https" || url.scheme() == "ftp" {
            return true;
        }
    }
    return false;
}
