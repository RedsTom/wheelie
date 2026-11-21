//! Release feed: reads the published Wheelie versions from GitHub.
//!
//! The request is best-effort. It runs off the UI thread, has a short timeout,
//! and any failure (offline, rate limit, changed payload) is reported so the
//! caller can log it and move on. Public betas are published as regular
//! releases; the feed tells them apart by their tag and only ever returns a
//! stable version.

use crate::core::{Release, VERSION, is_stable_tag};
use crate::platform::error::PlatformError;
use serde::Deserialize;
use std::time::Duration;

/// GitHub API feed listing the repository releases, newest first.
const RELEASES_URL: &str = "https://api.github.com/repos/RedsTom/wheelie/releases?per_page=100";

/// GitHub's shortcut for the newest non-draft, non-prerelease release.
const LATEST_URL: &str = "https://api.github.com/repos/RedsTom/wheelie/releases/latest";

/// Upper bound on paginated reads so a broken `Link` header cannot loop.
const MAX_PAGES: usize = 5;

/// Request budget for the startup feed read.
const TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Deserialize)]
struct RawRelease {
    tag_name: String,
    html_url: String,
    #[serde(default)]
    draft: bool,
}

impl RawRelease {
    fn stable(self) -> Option<Release> {
        if !self.draft && is_stable_tag(&self.tag_name) {
            Some(Release::new(self.tag_name, self.html_url))
        } else {
            None
        }
    }
}

/// Extracts the `rel="next"` target from a GitHub `Link` header value.
fn next_page(link: &str) -> Option<&str> {
    link.split(',').find_map(|entry| {
        let (target, params) = entry.split_once(';')?;
        if !params.contains("rel=\"next\"") && !params.contains("rel=next") {
            return None;
        }
        Some(target.trim().trim_start_matches('<').trim_end_matches('>'))
    })
}

/// Fetches the newest published stable release.
///
/// Errors are opaque on purpose: the caller never shows them to the user.
/// `/releases/latest` is tried first, with the paginated list as a complement
/// when the shortcut is missing or the tag is not a stable one.
pub fn latest_stable_release() -> Result<Release, PlatformError> {
    let config = ureq::Agent::config_builder()
        .user_agent(format!("Wheelie/{VERSION}"))
        .timeout_global(Some(TIMEOUT))
        .https_only(true)
        .max_redirects(3)
        .build();
    let agent = config.new_agent();

    if let Ok(mut response) = agent.get(LATEST_URL).call() {
        match response.body_mut().read_json::<RawRelease>() {
            Ok(raw) => {
                if let Some(release) = raw.stable() {
                    return Ok(release);
                }
            }
            Err(error) => {
                tracing::debug!(%error, "latest release shortcut unreadable; using the list");
            }
        }
    }

    let mut url = RELEASES_URL.to_owned();
    for _ in 0..MAX_PAGES {
        let mut response = agent.get(url.as_str()).call().map_err(|error| {
            PlatformError::Feed(format!("release feed request failed: {error}"))
        })?;
        let following = response
            .headers()
            .get("link")
            .and_then(|value| value.to_str().ok())
            .and_then(next_page)
            .map(str::to_owned);
        let releases: Vec<RawRelease> = response.body_mut().read_json().map_err(|error| {
            PlatformError::Feed(format!("release feed payload unreadable: {error}"))
        })?;

        if let Some(release) = releases.into_iter().find_map(RawRelease::stable) {
            return Ok(release);
        }
        match following {
            Some(next) => url = next,
            None => break,
        }
    }

    Err(PlatformError::Feed(
        "no published stable release".to_owned(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_link_is_parsed_from_github_headers() {
        let header = "<https://api.github.com/repos/RedsTom/wheelie/releases?page=2>; rel=\"next\", <https://api.github.com/repos/RedsTom/wheelie/releases?page=3>; rel=\"last\"";
        assert_eq!(
            next_page(header),
            Some("https://api.github.com/repos/RedsTom/wheelie/releases?page=2")
        );
    }

    #[test]
    fn a_header_without_a_next_page_is_ignored() {
        assert_eq!(
            next_page("<https://example.test/releases?page=9>; rel=\"last\""),
            None
        );
        assert_eq!(next_page(""), None);
    }
}
