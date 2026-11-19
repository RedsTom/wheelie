//! Release feed: reads the published Wheelie versions from GitHub.
//!
//! The request is best-effort. It runs off the UI thread, has a short timeout,
//! and any failure (offline, rate limit, changed payload) is reported so the
//! caller can log it and move on. Prereleases (public betas) are skipped: only
//! stable versions are announced inside the app.

use crate::core::{Release, VERSION};
use crate::platform::error::PlatformError;
use serde::Deserialize;
use std::time::Duration;

/// GitHub API feed listing the repository releases, newest first.
const RELEASES_URL: &str = "https://api.github.com/repos/RedsTom/wheelie/releases?per_page=30";

/// Request budget for the startup feed read.
const TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Deserialize)]
struct RawRelease {
    tag_name: String,
    html_url: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
}

/// Fetches the newest published stable release.
///
/// Errors are opaque on purpose: the caller never shows them to the user.
pub fn latest_stable_release() -> Result<Release, PlatformError> {
    let config = ureq::Agent::config_builder()
        .user_agent(format!("Wheelie/{VERSION}"))
        .timeout_global(Some(TIMEOUT))
        .https_only(true)
        .max_redirects(3)
        .build();

    let releases: Vec<RawRelease> = config
        .new_agent()
        .get(RELEASES_URL)
        .call()
        .map_err(|error| PlatformError::Other(format!("release feed request failed: {error}")))?
        .body_mut()
        .read_json()
        .map_err(|error| {
            PlatformError::Other(format!("release feed payload unreadable: {error}"))
        })?;

    releases
        .into_iter()
        .find(|release| !release.draft && !release.prerelease)
        .map(|release| Release::new(release.tag_name, release.html_url))
        .ok_or_else(|| PlatformError::Other("no published stable release".to_owned()))
}
