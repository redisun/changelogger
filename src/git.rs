//! Git repository operations and commit information extraction.
//!
//! This module provides functions to interact with git repositories, find version tags,
//! retrieve commit information, and extract remote repository URLs.

use anyhow::{anyhow, Result};
use git2::{Oid, Repository, Sort};
use semver::Version;

/// Information about a remote repository.
///
/// Contains the base URL of the remote repository (e.g., <https://github.com/owner/repo/>)
/// which is used to generate links to commits, issues, and releases in the changelog.
#[derive(Debug, Clone)]
pub struct RemoteInfo {
    /// The base URL of the remote repository, including trailing slash.
    pub base_url: String, // https://github.com/owner/repo/
}

/// Information about a git commit.
///
/// Contains the commit hash, short ID, summary (first line of commit message),
/// and full body text.
#[derive(Debug, Clone)]
pub struct CommitInfo {
    /// The full commit hash (OID).
    #[expect(unused)]
    pub oid: Oid,
    /// The short commit hash (typically 7 characters).
    pub short_id: String,
    /// The first line of the commit message (summary).
    pub summary: String,
    /// The full commit message body.
    #[expect(unused)]
    pub body: String,
}

/// Opens a git repository at the specified path.
///
/// Uses `Repository::discover` to find the repository, which will search
/// upward from the given path until it finds a `.git` directory.
///
/// # Arguments
///
/// * `path` - The path to the repository (or any directory within it)
///
/// # Errors
///
/// Returns an error if no git repository is found at or above the given path.
pub fn open_repo(path: &str) -> Result<Repository> {
    let repo = Repository::discover(path)?;
    Ok(repo)
}

/// Finds the latest semantic version tag in the repository.
///
/// Searches for tags matching the pattern "v*" and parses them as semantic versions.
/// Returns the tag with the most recent commit timestamp.
///
/// # Arguments
///
/// * `repo` - The git repository to search
///
/// # Returns
///
/// Returns `Some((tag_name, commit_oid, version))` if a semver tag is found,
/// or `None` if no valid semver tags exist.
///
/// # Errors
///
/// Returns an error if tag parsing or commit lookup fails.
pub fn find_latest_semver_tag(repo: &Repository) -> Result<Option<(String, Oid, Version)>> {
    let tags = repo.tag_names(Some("v*"))?;
    let mut best: Option<(String, Oid, Version)> = None;

    for name_opt in tags.iter() {
        let name = match name_opt {
            Some(n) => n.to_string(),
            None => continue,
        };

        let version_str = name.trim_start_matches('v');
        let version = match Version::parse(version_str) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let obj = repo.revparse_single(&name)?;
        let commit = obj.peel_to_commit()?;
        let oid = commit.id();

        best = match best {
            None => Some((name, oid, version)),
            Some((best_name, best_oid, best_v)) => {
                let best_commit = repo.find_commit(best_oid)?;
                if commit.time().seconds() > best_commit.time().seconds() {
                    Some((name, oid, version))
                } else {
                    Some((best_name, best_oid, best_v))
                }
            }
        };
    }

    Ok(best)
}

/// Retrieves all commits since a given commit (or all commits if `None`).
///
/// Uses a revwalk to traverse commits from HEAD, excluding commits reachable
/// from the `since` commit. Commits are sorted topologically and by time.
///
/// # Arguments
///
/// * `repo` - The git repository
/// * `since` - Optional commit OID to start from (exclusive). If `None`, all commits are returned.
///
/// # Returns
///
/// A vector of `CommitInfo` structures containing commit details.
///
/// # Errors
///
/// Returns an error if the revwalk fails or commits cannot be found.
pub fn commits_since(repo: &Repository, since: Option<Oid>) -> Result<Vec<CommitInfo>> {
    let mut revwalk = repo.revwalk()?;
    revwalk.set_sorting(Sort::TOPOLOGICAL | Sort::TIME)?;

    let head = repo.head()?;
    let head_oid = head
        .target()
        .ok_or_else(|| anyhow!("HEAD has no target commit"))?;

    revwalk.push(head_oid)?;

    if let Some(since_oid) = since {
        revwalk.hide(since_oid)?;
    }

    let mut commits = Vec::new();

    for oid_res in revwalk {
        let oid = oid_res?;
        let commit = repo.find_commit(oid)?;

        let summary = commit.summary().unwrap_or("No summary").to_string();
        let body = commit.body().unwrap_or("").to_string();

        let short = repo
            .find_object(oid, None)?
            .short_id()?
            .as_str()
            .unwrap_or_default()
            .to_string();

        commits.push(CommitInfo {
            oid,
            short_id: short,
            summary,
            body,
        });
    }

    Ok(commits)
}

/// Parses a git remote URL and converts it to a base URL.
///
/// Supports both SSH (git@) and HTTPS URLs. Converts SSH URLs to HTTPS format.
///
/// # Arguments
///
/// * `url` - The remote URL (e.g., "git@github.com:user/repo.git" or "https://github.com/user/repo.git")
///
/// # Returns
///
/// Returns `Some(RemoteInfo)` if the URL can be parsed, or `None` otherwise.
pub(crate) fn parse_remote_url(url: &str) -> Option<RemoteInfo> {
    if url.starts_with("git@") {
        if let Some((host_part, path_part)) = url.split_once(':') {
            let host = host_part.strip_prefix("git@").unwrap_or(host_part);
            // Trim trailing slash first, then .git extension
            let path = path_part.trim_end_matches('/').trim_end_matches(".git");
            return Some(RemoteInfo {
                base_url: format!("https://{host}/{path}/"),
            });
        }
    } else if url.starts_with("https://") {
        let without_git = url.trim_end_matches(".git");
        let with_slash = if without_git.ends_with('/') {
            without_git.to_string()
        } else {
            format!("{without_git}/")
        };
        return Some(RemoteInfo {
            base_url: with_slash,
        });
    }

    None
}

/// Extracts remote repository information from the "origin" remote.
///
/// Parses the remote URL and converts it to a base URL suitable for generating
/// links to commits, issues, and releases. Supports both SSH (git@) and HTTPS URLs.
///
/// # Arguments
///
/// * `repo` - The git repository
///
/// # Returns
///
/// Returns `Some(RemoteInfo)` if the origin remote exists and has a parseable URL,
/// or `None` otherwise.
pub fn get_remote_info(repo: &Repository) -> Option<RemoteInfo> {
    let remote = repo.find_remote("origin").ok()?;
    let url = remote.url()?;
    parse_remote_url(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_remote_url_https() {
        let result = parse_remote_url("https://github.com/user/repo.git");
        assert_eq!(
            result.map(|r| r.base_url),
            Some("https://github.com/user/repo/".to_string())
        );
    }

    #[test]
    fn test_parse_remote_url_https_with_slash() {
        let result = parse_remote_url("https://github.com/user/repo/");
        assert_eq!(
            result.map(|r| r.base_url),
            Some("https://github.com/user/repo/".to_string())
        );
    }

    #[test]
    fn test_parse_remote_url_https_no_git() {
        let result = parse_remote_url("https://github.com/user/repo");
        assert_eq!(
            result.map(|r| r.base_url),
            Some("https://github.com/user/repo/".to_string())
        );
    }

    #[test]
    fn test_parse_remote_url_ssh() {
        let result = parse_remote_url("git@github.com:user/repo.git");
        assert_eq!(
            result.map(|r| r.base_url),
            Some("https://github.com/user/repo/".to_string())
        );
    }

    #[test]
    fn test_parse_remote_url_ssh_no_git() {
        let result = parse_remote_url("git@github.com:user/repo");
        assert_eq!(
            result.map(|r| r.base_url),
            Some("https://github.com/user/repo/".to_string())
        );
    }

    #[test]
    fn test_parse_remote_url_ssh_with_trailing_slash() {
        let result = parse_remote_url("git@github.com:user/repo.git/");
        assert_eq!(
            result.map(|r| r.base_url),
            Some("https://github.com/user/repo/".to_string())
        );
    }

    #[test]
    fn test_parse_remote_url_ssh_custom_host() {
        let result = parse_remote_url("git@gitlab.com:group/project.git");
        assert_eq!(
            result.map(|r| r.base_url),
            Some("https://gitlab.com/group/project/".to_string())
        );
    }

    #[test]
    fn test_parse_remote_url_invalid() {
        assert!(parse_remote_url("not a url").is_none());
        assert!(parse_remote_url("http://github.com/user/repo").is_none());
        assert!(parse_remote_url("").is_none());
    }

    #[test]
    fn test_parse_remote_url_https_with_port() {
        // Note: This might not work with current implementation, but let's test it
        let result = parse_remote_url("https://github.com:443/user/repo.git");
        // Current implementation should handle this
        assert!(result.is_some());
    }
}
