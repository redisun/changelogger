//! Changelog generation and formatting functionality.
//!
//! This module provides functions to build changelog sections from commit information
//! and write them to files.

use std::collections::HashMap;
use std::fmt::Write;
use std::fs;
use std::path::Path;

use anyhow::Result;
use chrono::NaiveDate;
use regex::Regex;
use semver::Version;

use crate::classify::CommitCategory;
use crate::git::{CommitInfo, RemoteInfo};

/// Builds a markdown-formatted release section for a changelog.
///
/// Creates a version header with optional links to the remote repository,
/// formats commits by category (breaking changes, new features, bug fixes),
/// and includes links to commits and issues when remote information is available.
///
/// # Arguments
///
/// * `new_version` - The version number for this release
/// * `last_version` - The previous version number
/// * `date` - The release date
/// * `remote` - Optional remote repository information for generating links
/// * `grouped` - Commits grouped by category (Major, Minor, Patch)
///
/// # Returns
///
/// A markdown-formatted string containing the release section.
pub fn build_release_section(
    new_version: &Version,
    last_version: &Version,
    date: NaiveDate,
    remote: Option<&RemoteInfo>,
    grouped: &HashMap<CommitCategory, Vec<CommitInfo>>,
) -> String {
    let date_str = date.format("%Y-%m-%d").to_string();
    let mut out = String::new();

    let version_str = new_version.to_string();
    let last_str = last_version.to_string();

    let header = if let Some(r) = remote {
        format!(
            "## [Version {version_str}]({}releases/tag/v{version_str}) ({date_str})\n",
            r.base_url
        )
    } else {
        format!("## Version {version_str} ({date_str})\n")
    };
    out.push_str(&header);

    if let Some(list) = grouped.get(&CommitCategory::Major) {
        out.push_str(&format_section("Breaking changes", list, remote));
    }
    if let Some(list) = grouped.get(&CommitCategory::Minor) {
        out.push_str(&format_section("New features", list, remote));
    }
    if let Some(list) = grouped.get(&CommitCategory::Patch) {
        out.push_str(&format_section("Bug fixes", list, remote));
    }

    if let Some(r) = remote {
        if last_str != "0.0.0" {
            out.push_str(&format!(
                "\n[...full changes]({}compare/v{last_str}...v{version_str})\n\n",
                r.base_url
            ));
        } else {
            out.push('\n');
        }
    } else {
        out.push('\n');
    }

    out
}

/// Formats a section of commits (e.g., "Breaking changes", "New features", "Bug fixes").
///
/// Extracts issue references from commit messages and formats them as markdown list items
/// with links to commits and issues when remote information is available.
///
/// # Arguments
///
/// * `heading` - The section heading (e.g., "Breaking changes")
/// * `commits` - The list of commits to format
/// * `remote` - Optional remote repository information for generating links
///
/// # Returns
///
/// A markdown-formatted string containing the section.
fn format_section(heading: &str, commits: &[CommitInfo], remote: Option<&RemoteInfo>) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "\n### {heading}");

    static RE_SQUASHED: once_cell::sync::Lazy<Regex> =
        once_cell::sync::Lazy::new(|| Regex::new(r"\s+\(#(\d+)\)").unwrap());
    static RE_TRAILING: once_cell::sync::Lazy<Regex> =
        once_cell::sync::Lazy::new(|| Regex::new(r"\s+#(\d+)$").unwrap());

    for commit in commits {
        let mut title = commit.summary.clone();
        let mut issue_id: Option<String> = None;

        if let Some(cap) = RE_SQUASHED.captures(&title) {
            if let Some(m) = cap.get(1) {
                issue_id = Some(m.as_str().to_string());
            }
            title = RE_SQUASHED.replace(&title, "").into_owned();
        }

        if issue_id.is_none() {
            if let Some(cap) = RE_TRAILING.captures(&title) {
                if let Some(m) = cap.get(1) {
                    issue_id = Some(m.as_str().to_string());
                }
                title = RE_TRAILING.replace(&title, "").into_owned();
            }
        }

        let issue_ref = if let (Some(r), Some(id)) = (remote, issue_id.as_ref()) {
            format!(" ([#{id}]({}issues/{id}))", r.base_url)
        } else if let Some(id) = issue_id {
            format!(" (#{id})")
        } else {
            String::new()
        };

        let commit_ref = if let Some(r) = remote {
            format!(
                " [`{}`]({}commit/{})",
                commit.short_id, r.base_url, commit.short_id
            )
        } else {
            format!(" `{}`", commit.short_id)
        };

        out.push_str("* ");
        out.push_str(&title);
        out.push(':');
        out.push_str(&commit_ref);
        out.push_str(&issue_ref);
        out.push('\n');
    }

    out.push('\n');
    out
}

/// Writes a new changelog section to a file.
///
/// If the file exists and contains content, the new section is prepended.
/// If the file doesn't exist or is empty, a new changelog is created with a footer.
///
/// # Arguments
///
/// * `path` - The path to the changelog file
/// * `new_section` - The new release section to add
///
/// # Errors
///
/// Returns an error if the file cannot be read or written.
pub fn write_changelog(path: &str, new_section: &str) -> Result<()> {
    let p = Path::new(path);

    let existing = if p.exists() {
        fs::read_to_string(p)?
    } else {
        String::new()
    };

    let content = if existing.trim().is_empty() {
        format!("{new_section}\n--- Generated by changelogger\n")
    } else {
        format!("{new_section}\n\n{existing}")
    };

    fs::write(p, content)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::Oid;
    use std::collections::HashMap;
    use tempfile::TempDir;

    fn create_commit_info(short_id: &str, summary: &str) -> CommitInfo {
        CommitInfo {
            oid: Oid::zero(),
            short_id: short_id.to_string(),
            summary: summary.to_string(),
            body: String::new(),
        }
    }

    fn create_remote_info(base_url: &str) -> RemoteInfo {
        RemoteInfo {
            base_url: base_url.to_string(),
        }
    }

    #[test]
    fn test_build_release_section_no_remote() {
        let new_version = Version::parse("1.2.3").unwrap();
        let last_version = Version::parse("1.2.2").unwrap();
        let date = NaiveDate::from_ymd_opt(2024, 1, 15).unwrap();
        let mut grouped = HashMap::new();

        grouped.insert(
            CommitCategory::Patch,
            vec![create_commit_info("abc1234", "fix: bug fix")],
        );

        let result = build_release_section(&new_version, &last_version, date, None, &grouped);

        assert!(result.contains("## Version 1.2.3 (2024-01-15)"));
        assert!(result.contains("### Bug fixes"));
        assert!(result.contains("* fix: bug fix:"));
        assert!(result.contains("`abc1234`"));
    }

    #[test]
    fn test_build_release_section_with_remote() {
        let new_version = Version::parse("2.0.0").unwrap();
        let last_version = Version::parse("1.9.9").unwrap();
        let date = NaiveDate::from_ymd_opt(2024, 2, 20).unwrap();
        let remote = create_remote_info("https://github.com/user/repo/");
        let mut grouped = HashMap::new();

        grouped.insert(
            CommitCategory::Major,
            vec![create_commit_info("def5678", "breaking: remove old API")],
        );

        let result =
            build_release_section(&new_version, &last_version, date, Some(&remote), &grouped);

        assert!(
            result.contains("## [Version 2.0.0](https://github.com/user/repo/releases/tag/v2.0.0)")
        );
        assert!(result.contains("### Breaking changes"));
        assert!(result.contains("[`def5678`](https://github.com/user/repo/commit/def5678)"));
        assert!(result
            .contains("[...full changes](https://github.com/user/repo/compare/v1.9.9...v2.0.0)"));
    }

    #[test]
    fn test_build_release_section_all_categories() {
        let new_version = Version::parse("1.5.0").unwrap();
        let last_version = Version::parse("1.4.0").unwrap();
        let date = NaiveDate::from_ymd_opt(2024, 3, 10).unwrap();
        let mut grouped = HashMap::new();

        grouped.insert(
            CommitCategory::Major,
            vec![create_commit_info("maj1", "breaking: change")],
        );
        grouped.insert(
            CommitCategory::Minor,
            vec![create_commit_info("min1", "feat: new feature")],
        );
        grouped.insert(
            CommitCategory::Patch,
            vec![create_commit_info("pat1", "fix: bug")],
        );

        let result = build_release_section(&new_version, &last_version, date, None, &grouped);

        assert!(result.contains("### Breaking changes"));
        assert!(result.contains("### New features"));
        assert!(result.contains("### Bug fixes"));
        assert!(result.contains("breaking: change"));
        assert!(result.contains("feat: new feature"));
        assert!(result.contains("fix: bug"));
    }

    #[test]
    fn test_build_release_section_initial_version() {
        let new_version = Version::parse("1.0.0").unwrap();
        let last_version = Version::parse("0.0.0").unwrap();
        let date = NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let remote = create_remote_info("https://github.com/user/repo/");
        let mut grouped = HashMap::new();

        grouped.insert(
            CommitCategory::Minor,
            vec![create_commit_info("init1", "feat: initial release")],
        );

        let result =
            build_release_section(&new_version, &last_version, date, Some(&remote), &grouped);

        // Should not include compare link for 0.0.0
        assert!(!result.contains("compare/v0.0.0"));
    }

    #[test]
    fn test_format_section_with_issue_references() {
        let remote = create_remote_info("https://github.com/user/repo/");
        let commits = vec![
            create_commit_info("abc123", "fix: bug (#42)"),
            create_commit_info("def456", "feat: feature (#123)"),
            create_commit_info("ghi789", "fix: another bug"),
        ];

        let result = format_section("Bug fixes", &commits, Some(&remote));

        assert!(result.contains("### Bug fixes"));
        assert!(result.contains("fix: bug:"));
        assert!(result.contains("([#42](https://github.com/user/repo/issues/42))"));
        assert!(result.contains("([#123](https://github.com/user/repo/issues/123))"));
        assert!(result.contains("fix: another bug:"));
        assert!(!result.contains("(#"));
    }

    #[test]
    fn test_format_section_squashed_issue_format() {
        let remote = create_remote_info("https://github.com/user/repo/");
        let commits = vec![create_commit_info("abc123", "fix: bug (#99)")];

        let result = format_section("Bug fixes", &commits, Some(&remote));

        assert!(result.contains("fix: bug:"));
        assert!(result.contains("([#99](https://github.com/user/repo/issues/99))"));
    }

    #[test]
    fn test_format_section_no_remote() {
        let commits = vec![
            create_commit_info("abc123", "fix: bug (#42)"),
            create_commit_info("def456", "feat: feature"),
        ];

        let result = format_section("Changes", &commits, None);

        assert!(result.contains("### Changes"));
        assert!(result.contains("fix: bug:"));
        assert!(result.contains("(#42)")); // Issue number without link
        assert!(result.contains("`abc123`")); // Commit hash without link
        assert!(result.contains("feat: feature:"));
    }

    #[test]
    fn test_write_changelog_new_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("CHANGELOG.md");
        let section = "## Version 1.0.0 (2024-01-01)\n\n### Bug fixes\n* fix: bug\n\n";

        write_changelog(file_path.to_str().unwrap(), section).unwrap();

        let content = fs::read_to_string(&file_path).unwrap();
        assert!(content.contains(section));
        assert!(content.contains("--- Generated by changelogger"));
    }

    #[test]
    fn test_write_changelog_existing_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("CHANGELOG.md");
        let existing = "## Version 0.9.0 (2023-12-01)\n\n### Bug fixes\n* old fix\n\n";
        fs::write(&file_path, existing).unwrap();

        let new_section = "## Version 1.0.0 (2024-01-01)\n\n### Bug fixes\n* new fix\n\n";
        write_changelog(file_path.to_str().unwrap(), new_section).unwrap();

        let content = fs::read_to_string(&file_path).unwrap();
        assert!(content.starts_with(new_section.trim()));
        assert!(content.contains(existing.trim()));
    }

    #[test]
    fn test_write_changelog_empty_existing_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("CHANGELOG.md");
        fs::write(&file_path, "   \n\n  ").unwrap(); // Whitespace only

        let section = "## Version 1.0.0 (2024-01-01)\n\n### Bug fixes\n* fix\n\n";
        write_changelog(file_path.to_str().unwrap(), section).unwrap();

        let content = fs::read_to_string(&file_path).unwrap();
        assert!(content.contains(section));
        assert!(content.contains("--- Generated by changelogger"));
    }
}
