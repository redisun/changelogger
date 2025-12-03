//! Commit classification and categorization functionality.
//!
//! This module provides functions to automatically classify commits into categories
//! (Major, Minor, Patch, Ignore) based on commit message conventions and patterns.

use regex::Regex;
use semver::Version;

use crate::git::CommitInfo;

/// Categories for classifying commits based on their impact.
///
/// Commits are classified to determine the appropriate version bump:
/// - `Major`: Breaking changes that require a major version bump
/// - `Minor`: New features that require a minor version bump
/// - `Patch`: Bug fixes and small changes that require a patch version bump
/// - `Ignore`: Commits that should not appear in the changelog (docs, style, etc.)
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum CommitCategory {
    Major,
    Minor,
    Patch,
    /// Commits that should be ignored (not included in changelog).
    Ignore,
}

/// Checks if a commit message is a release message.
///
/// Release messages follow the format "-> v1.2.3" or "-> 1.2.3".
/// These commits are typically used to mark releases and should be ignored.
///
/// # Arguments
///
/// * `subject` - The commit message subject line
///
/// # Returns
///
/// Returns `Some(Version)` if the message is a release message, or `None` otherwise.
pub fn is_release_message(subject: &str) -> Option<Version> {
    if let Some(rest) = subject.strip_prefix("-> ") {
        if let Ok(v) = Version::parse(rest.trim_start_matches('v')) {
            return Some(v);
        }
    }
    None
}

/// Maps a commit message prefix to a commit category.
///
/// Supports conventional commit prefixes like "feat", "fix", "docs", etc.
/// and maps them to appropriate categories.
///
/// # Arguments
///
/// * `prefix` - The commit message prefix (case-insensitive)
///
/// # Returns
///
/// Returns `Some(CommitCategory)` if the prefix is recognized, or `None` otherwise.
fn prefix_mapping(prefix: &str) -> Option<CommitCategory> {
    let cat = match () {
        _ if prefix.eq_ignore_ascii_case("docs")
            || prefix.eq_ignore_ascii_case("doc")
            || prefix.eq_ignore_ascii_case("style")
            || prefix.eq_ignore_ascii_case("chore")
            || prefix.eq_ignore_ascii_case("test") =>
        {
            CommitCategory::Ignore
        }
        _ if prefix.eq_ignore_ascii_case("tweak") || prefix.eq_ignore_ascii_case("tweaks") => {
            CommitCategory::Patch
        }
        _ if prefix.eq_ignore_ascii_case("fix")
            || prefix.eq_ignore_ascii_case("fixes")
            || prefix.eq_ignore_ascii_case("perf")
            || prefix.eq_ignore_ascii_case("refactor")
            || prefix.eq_ignore_ascii_case("patch") =>
        {
            CommitCategory::Patch
        }
        _ if prefix.eq_ignore_ascii_case("feat") || prefix.eq_ignore_ascii_case("minor") => {
            CommitCategory::Minor
        }
        _ if prefix.eq_ignore_ascii_case("breaking") || prefix.eq_ignore_ascii_case("major") => {
            CommitCategory::Major
        }
        _ => return None,
    };
    Some(cat)
}

/// Automatically classifies a commit based on its message.
///
/// Analyzes the commit summary to determine its category. Supports:
/// - Conventional commit format: "type: subject" or "type(scope): subject"
/// - Release messages: "-> v1.2.3"
/// - Simple keywords: "tweak", "tweaks"
///
/// If a prefix is found and recognized, it is removed from the commit summary.
///
/// # Arguments
///
/// * `commit` - The commit to classify (summary may be modified)
///
/// # Returns
///
/// Returns `Some(CommitCategory)` if the commit can be automatically classified,
/// or `None` if manual classification is needed.
pub fn auto_classify(commit: &mut CommitInfo) -> Option<CommitCategory> {
    if is_release_message(&commit.summary).is_some() {
        return Some(CommitCategory::Ignore);
    }

    if commit.summary.eq_ignore_ascii_case("tweak") || commit.summary.eq_ignore_ascii_case("tweaks")
    {
        return Some(CommitCategory::Patch);
    }

    // type: subject
    // or type(scope): subject
    // Check scoped format first to avoid matching it with the simple format
    static RE_SCOPE: once_cell::sync::Lazy<Regex> =
        once_cell::sync::Lazy::new(|| Regex::new(r"^([^(]+)\([^)]+\):\s+").unwrap());
    static RE: once_cell::sync::Lazy<Regex> =
        once_cell::sync::Lazy::new(|| Regex::new(r"^([^:]+):\s+").unwrap());

    if let Some(cap) = RE_SCOPE.captures(&commit.summary) {
        if let Some(ty) = cap.get(1) {
            if let Some(cat) = prefix_mapping(ty.as_str()) {
                commit.summary = RE_SCOPE.replace(&commit.summary, "").into_owned();
                return Some(cat);
            }
        }
    } else if let Some(cap) = RE.captures(&commit.summary) {
        if let Some(ty) = cap.get(1) {
            if let Some(cat) = prefix_mapping(ty.as_str()) {
                commit.summary = RE.replace(&commit.summary, "").into_owned();
                return Some(cat);
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::Oid;

    fn create_commit_info(summary: &str) -> CommitInfo {
        CommitInfo {
            oid: Oid::zero(),
            short_id: "abc1234".to_string(),
            summary: summary.to_string(),
            body: String::new(),
        }
    }

    #[test]
    fn test_is_release_message() {
        assert_eq!(
            is_release_message("-> v1.2.3"),
            Some(Version::parse("1.2.3").unwrap())
        );
        assert_eq!(
            is_release_message("-> 1.2.3"),
            Some(Version::parse("1.2.3").unwrap())
        );
        assert_eq!(
            is_release_message("-> v0.1.0"),
            Some(Version::parse("0.1.0").unwrap())
        );
        assert_eq!(is_release_message("-> invalid"), None);
        assert_eq!(is_release_message("not a release"), None);
        assert_eq!(is_release_message("->"), None);
    }

    #[test]
    fn test_auto_classify_release_message() {
        let mut commit = create_commit_info("-> v1.2.3");
        assert_eq!(auto_classify(&mut commit), Some(CommitCategory::Ignore));
    }

    #[test]
    fn test_auto_classify_tweak() {
        let mut commit = create_commit_info("tweak");
        assert_eq!(auto_classify(&mut commit), Some(CommitCategory::Patch));
        assert_eq!(commit.summary, "tweak");

        let mut commit = create_commit_info("Tweaks");
        assert_eq!(auto_classify(&mut commit), Some(CommitCategory::Patch));
    }

    #[test]
    fn test_auto_classify_conventional_commits() {
        // Major
        let mut commit = create_commit_info("breaking: remove deprecated API");
        assert_eq!(auto_classify(&mut commit), Some(CommitCategory::Major));
        assert_eq!(commit.summary, "remove deprecated API");

        let mut commit = create_commit_info("major: breaking change");
        assert_eq!(auto_classify(&mut commit), Some(CommitCategory::Major));

        // Minor
        let mut commit = create_commit_info("feat: add new feature");
        assert_eq!(auto_classify(&mut commit), Some(CommitCategory::Minor));
        assert_eq!(commit.summary, "add new feature");

        let mut commit = create_commit_info("minor: add something");
        assert_eq!(auto_classify(&mut commit), Some(CommitCategory::Minor));

        // Patch
        let mut commit = create_commit_info("fix: resolve bug");
        assert_eq!(auto_classify(&mut commit), Some(CommitCategory::Patch));
        assert_eq!(commit.summary, "resolve bug");

        let mut commit = create_commit_info("perf: improve performance");
        assert_eq!(auto_classify(&mut commit), Some(CommitCategory::Patch));

        let mut commit = create_commit_info("refactor: clean up code");
        assert_eq!(auto_classify(&mut commit), Some(CommitCategory::Patch));

        // Ignore
        let mut commit = create_commit_info("docs: update README");
        assert_eq!(auto_classify(&mut commit), Some(CommitCategory::Ignore));
        assert_eq!(commit.summary, "update README");

        let mut commit = create_commit_info("style: format code");
        assert_eq!(auto_classify(&mut commit), Some(CommitCategory::Ignore));

        let mut commit = create_commit_info("chore: update dependencies");
        assert_eq!(auto_classify(&mut commit), Some(CommitCategory::Ignore));

        let mut commit = create_commit_info("test: add unit tests");
        assert_eq!(auto_classify(&mut commit), Some(CommitCategory::Ignore));
    }

    #[test]
    fn test_auto_classify_with_scope() {
        let mut commit = create_commit_info("feat(api): add new endpoint");
        assert_eq!(auto_classify(&mut commit), Some(CommitCategory::Minor));
        assert_eq!(commit.summary, "add new endpoint");

        let mut commit = create_commit_info("fix(parser): handle edge case");
        assert_eq!(auto_classify(&mut commit), Some(CommitCategory::Patch));
        assert_eq!(commit.summary, "handle edge case");

        let mut commit = create_commit_info("breaking(api): remove old method");
        assert_eq!(auto_classify(&mut commit), Some(CommitCategory::Major));
        assert_eq!(commit.summary, "remove old method");
    }

    #[test]
    fn test_auto_classify_case_insensitive() {
        let mut commit = create_commit_info("FEAT: uppercase");
        assert_eq!(auto_classify(&mut commit), Some(CommitCategory::Minor));

        let mut commit = create_commit_info("Fix: mixed case");
        assert_eq!(auto_classify(&mut commit), Some(CommitCategory::Patch));

        let mut commit = create_commit_info("DOCS: documentation");
        assert_eq!(auto_classify(&mut commit), Some(CommitCategory::Ignore));
    }

    #[test]
    fn test_auto_classify_unknown_prefix() {
        let mut commit = create_commit_info("unknown: something");
        assert_eq!(auto_classify(&mut commit), None);
        assert_eq!(commit.summary, "unknown: something");
    }

    #[test]
    fn test_auto_classify_no_prefix() {
        let mut commit = create_commit_info("just a regular commit message");
        assert_eq!(auto_classify(&mut commit), None);
        assert_eq!(commit.summary, "just a regular commit message");
    }

    #[test]
    fn test_auto_classify_multiple_colons() {
        let mut commit = create_commit_info("fix: handle error: invalid input");
        assert_eq!(auto_classify(&mut commit), Some(CommitCategory::Patch));
        assert_eq!(commit.summary, "handle error: invalid input");
    }
}
