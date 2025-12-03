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
