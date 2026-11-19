//! Update-availability rules for release notices.
//!
//! The rules are pure: fetching the published releases is a platform
//! concern, deciding whether one of them deserves a notice is domain logic.

use semver::Version;

/// A published release, as offered by the release feed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    /// Release tag, e.g. `v0.3.1`.
    pub tag: String,
    /// GitHub page for the release.
    pub url: String,
}

impl Release {
    /// Creates a release notice from a tag and its page.
    pub fn new(tag: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            tag: tag.into(),
            url: url.into(),
        }
    }
}

/// Parses a version string, tolerating a leading `v`.
fn parse_version(text: &str) -> Option<Version> {
    let trimmed = text.trim();
    let trimmed = trimmed.strip_prefix(['v', 'V']).unwrap_or(trimmed);
    Version::parse(trimmed).ok()
}

/// True when `tag` names a stable version, e.g. `v0.3.1`, and not a
/// prerelease such as `v0.3.2-beta.1`.
///
/// Feed filtering keys on the tag itself: public betas are published as
/// regular GitHub releases, and their flags say nothing about their nature.
/// Unparsable tags are not stable.
pub fn is_stable_tag(tag: &str) -> bool {
    parse_version(tag).is_some_and(|version| version.pre.is_empty())
}

/// True when `candidate` is a stable release strictly newer than `current`.
///
/// Prerelease candidates never count: betas are not announced inside the app.
/// Unparseable versions are treated as "no update".
pub fn is_update_available(current: &str, candidate: &str) -> bool {
    let (Some(current), Some(candidate)) = (parse_version(current), parse_version(candidate))
    else {
        return false;
    };
    candidate > current && candidate.pre.is_empty()
}

/// True when the release deserves a notice for this application run.
///
/// The release must be newer than the running version and must not be the
/// version the user chose to ignore.
pub fn should_notify(release: &Release, current: &str, ignored: Option<&str>) -> bool {
    if !is_update_available(current, &release.tag) {
        return false;
    }
    match ignored.and_then(parse_version) {
        Some(ignored) => Some(ignored) != parse_version(&release.tag),
        None => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release(tag: &str) -> Release {
        Release::new(tag, "https://github.com/RedsTom/wheelie/releases")
    }

    #[test]
    fn stable_and_prerelease_tags_are_told_apart() {
        assert!(is_stable_tag("v0.3.1"));
        assert!(is_stable_tag("0.3.2"));
        assert!(!is_stable_tag("v0.3.2-beta.1"));
        assert!(!is_stable_tag("0.4.0-rc.2"));
        assert!(!is_stable_tag("nightly"));
    }

    #[test]
    fn newer_stable_release_is_available() {
        assert!(is_update_available("0.3.1", "0.3.2"));
        assert!(is_update_available("0.3.1", "0.4.0"));
        assert!(is_update_available("0.9.9", "1.0.0"));
        assert!(is_update_available("v0.3.1", "v0.3.2"));
    }

    #[test]
    fn same_or_older_release_is_not_available() {
        assert!(!is_update_available("0.3.2", "0.3.2"));
        assert!(!is_update_available("0.3.2", "v0.3.1"));
        assert!(!is_update_available("0.4.0", "0.3.9"));
    }

    #[test]
    fn prerelease_candidates_are_never_announced() {
        assert!(!is_update_available("0.3.1", "0.4.0-beta.1"));
        assert!(!is_update_available("0.4.0-beta.1", "0.4.0-beta.2"));
    }

    #[test]
    fn running_a_beta_still_earns_the_stable_it_becomes() {
        assert!(is_update_available("0.3.2-beta.1", "0.3.2"));
        // Older stables do not look newer than a future beta.
        assert!(!is_update_available("0.3.2-beta.1", "0.3.1"));
    }

    #[test]
    fn unparsable_versions_are_not_updates() {
        assert!(!is_update_available("0.3.1", "nightly"));
        assert!(!is_update_available("", "0.3.2"));
    }

    #[test]
    fn ignored_version_is_not_announced_again() {
        assert!(!should_notify(&release("v0.3.2"), "0.3.1", Some("v0.3.2")));
        assert!(should_notify(&release("v0.3.2"), "0.3.1", Some("0.3.1")));
        assert!(should_notify(&release("v0.3.2"), "0.3.1", None));
    }

    #[test]
    fn unignored_or_unparsable_ignore_entries_do_not_suppress() {
        assert!(should_notify(&release("v0.3.2"), "0.3.1", Some("nonsense")));
        assert!(!should_notify(&release("v0.3.0"), "0.3.1", None));
    }
}
