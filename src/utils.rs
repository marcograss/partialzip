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

#[cfg(test)]
mod tests {
    #[test]
    pub fn ok_urls() {
        let ok = [
            "http://www.test.com",
            "https://sub.test.com",
            "ftp://ftp.test.com",
        ];
        let not_ok = [
            "file://repo.test.com",
            "asdasd://",
            "js:",
            "smb://storage.test.com",
        ];
        for s in ok {
            assert!(super::url_is_valid(s), "{} should be a valid url", s);
        }
        for s in not_ok {
            assert!(!super::url_is_valid(s), "{} should be a invalid url", s)
        }
    }
}
