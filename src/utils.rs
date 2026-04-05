use url::Url;

/// Returns if a URL is a valid URL string supported by the library
#[must_use]
pub fn url_is_valid(url: &str) -> bool {
    Url::parse(url).is_ok_and(|url| {
        // Supported URL schemes
        ["http", "https", "ftp", "file"].contains(&url.scheme())
    })
}

/// Matches a string against a glob pattern.
///
/// Supports `*` (matches any sequence of characters, including empty) and
/// `?` (matches any single character).
#[must_use]
pub fn glob_match(pattern: &str, text: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let t: Vec<char> = text.chars().collect();
    let (plen, tlen) = (p.len(), t.len());

    // dp[i][j] = pattern[0..i] matches text[0..j]
    let mut dp = vec![vec![false; tlen + 1]; plen + 1];
    dp[0][0] = true;

    for i in 1..=plen {
        if p[i - 1] == '*' {
            dp[i][0] = dp[i - 1][0];
            for j in 1..=tlen {
                dp[i][j] = dp[i - 1][j] || dp[i][j - 1];
            }
        } else {
            for j in 1..=tlen {
                dp[i][j] = dp[i - 1][j - 1] && (p[i - 1] == '?' || p[i - 1] == t[j - 1]);
            }
        }
    }

    dp[plen][tlen]
}
