//! Main entry point for the changelogger application.
//!
//! This module handles command-line argument parsing, orchestrates the changelog
//! generation process, and provides interactive classification of commits.

use std::collections::HashMap;

use anyhow::{anyhow, Context, Result};
use chrono::Local;
use clap::Parser;
use dialoguer::{theme::ColorfulTheme, Select};
use owo_colors::OwoColorize;
use semver::Version;

mod changelog;
mod classify;
mod git;

use changelog::{build_release_section, write_changelog};
use classify::{auto_classify, CommitCategory};
use git::{commits_since, find_latest_semver_tag, get_remote_info, open_repo, CommitInfo};

/// Command-line interface arguments for changelogger.
#[derive(Parser, Debug)]
#[command(
    name = "changelogger",
    version,
    about = "Generate or update CHANGELOG.md from git commits"
)]
struct Cli {
    /// Path to the repository, defaults to current directory
    #[arg(long, default_value = ".")]
    repo: String,

    /// Optional new version, otherwise computed from commits
    #[arg(long)]
    new_version: Option<String>,

    /// Optional tag to start from, otherwise latest semver tag is used
    #[arg(long)]
    from_tag: Option<String>,

    /// File to write the changelog to
    #[arg(long, default_value = "CHANGELOG.md")]
    output: String,

    /// Dry run, print to stdout instead of writing file
    #[arg(long)]
    dry_run: bool,

    /// Do not ask interactive questions, unknown commits become patch by default
    #[arg(long)]
    non_interactive: bool,
}

/// Main entry point for the changelogger application.
///
/// Processes command-line arguments, opens the git repository, finds commits since
/// the last version tag, classifies commits (interactively or automatically),
/// determines the new version number, and generates/updates the changelog file.
///
/// # Errors
///
/// Returns an error if:
/// - The repository cannot be opened
/// - No commits are found since the starting point
/// - Version parsing fails
/// - The changelog file cannot be written
fn main() -> Result<()> {
    let cli = Cli::parse();

    let repo = open_repo(&cli.repo)
        .with_context(|| format!("Could not open git repository at {}", cli.repo))?;
    println!("{}", "Opened repository".cyan());

    let (last_version, since_oid) = if let Some(tag_name) = cli.from_tag {
        let obj = repo
            .revparse_single(&tag_name)
            .with_context(|| format!("Could not find tag {tag_name}"))?;
        let commit = obj.peel_to_commit()?;
        let version_str = tag_name.trim_start_matches('v');
        let version = Version::parse(version_str)
            .with_context(|| format!("Tag {tag_name} does not look like a semver version"))?;
        (version, Some(commit.id()))
    } else if let Some((tag, oid, v)) = find_latest_semver_tag(&repo)? {
        println!(
            "{} latest tag is {} (commit {})",
            "Info".bright_blue(),
            tag,
            oid
        );
        (v, Some(oid))
    } else {
        println!(
            "{} no semver git tags found, assuming previous version 0.0.0 and using full history",
            "Info".bright_blue()
        );
        (Version::parse("0.0.0")?, None)
    };

    let commits = commits_since(&repo, since_oid)?;
    if commits.is_empty() {
        return Err(anyhow!("No commits found since starting point"));
    }

    let mut classified: Vec<(CommitInfo, Option<CommitCategory>)> = commits
        .into_iter()
        .map(|mut c| {
            let cat = auto_classify(&mut c);
            (c, cat)
        })
        .collect();

    if !cli.non_interactive {
        static ITEMS: &[&str] = &["patch", "minor", "major", "ignore"];
        let theme = ColorfulTheme::default();
        for (commit, cat) in classified.iter_mut() {
            if cat.is_some() {
                continue;
            }

            println!(
                "\n{} {} {}",
                "Commit".bold(),
                commit.short_id.yellow(),
                commit.summary.bold()
            );

            let choice = Select::with_theme(&theme)
                .with_prompt("Select type")
                .items(ITEMS)
                .default(0)
                .interact()
                .unwrap_or(0);

            let selected = match ITEMS[choice] {
                "patch" => CommitCategory::Patch,
                "minor" => CommitCategory::Minor,
                "major" => CommitCategory::Major,
                _ => CommitCategory::Ignore,
            };

            *cat = Some(selected);
        }
    }

    if cli.non_interactive {
        for (_, cat) in classified.iter_mut() {
            if cat.is_none() {
                *cat = Some(CommitCategory::Patch);
            }
        }
    }

    let mut grouped: HashMap<CommitCategory, Vec<CommitInfo>> = HashMap::new();
    for (commit, cat_opt) in classified.into_iter() {
        if let Some(cat) = cat_opt {
            if cat == CommitCategory::Ignore {
                continue;
            }
            grouped.entry(cat).or_default().push(commit);
        }
    }

    if !grouped.contains_key(&CommitCategory::Major)
        && !grouped.contains_key(&CommitCategory::Minor)
        && !grouped.contains_key(&CommitCategory::Patch)
    {
        return Err(anyhow!(
            "No important commits found, nothing to put into changelog"
        ));
    }

    let new_version = if let Some(v) = cli.new_version {
        let parsed = Version::parse(&v)
            .with_context(|| format!("Provided version {v} is not valid semver"))?;
        if parsed <= last_version {
            return Err(anyhow!(
                "New version {} must be greater than previous version {}",
                parsed,
                last_version
            ));
        }
        parsed
    } else {
        let unstable = last_version < Version::new(1, 0, 0);
        let has_major = grouped.contains_key(&CommitCategory::Major);
        let has_minor = grouped.contains_key(&CommitCategory::Minor);

        if has_major {
            if unstable {
                Version::new(last_version.major, last_version.minor + 1, 0)
            } else {
                Version::new(last_version.major + 1, 0, 0)
            }
        } else if has_minor {
            if unstable {
                Version::new(
                    last_version.major,
                    last_version.minor,
                    last_version.patch + 1,
                )
            } else {
                Version::new(last_version.major, last_version.minor + 1, 0)
            }
        } else {
            Version::new(
                last_version.major,
                last_version.minor,
                last_version.patch + 1,
            )
        }
    };

    println!(
        "{} previous version {} -> new version {}",
        "Version".green(),
        last_version,
        new_version
    );

    let remote_info = get_remote_info(&repo);
    let today = Local::now().date_naive();

    let section = build_release_section(
        &new_version,
        &last_version,
        today,
        remote_info.as_ref(),
        &grouped,
    );

    if cli.dry_run {
        println!("\n{}", section);
    } else {
        write_changelog(&cli.output, &section)?;
        println!("{} updated {}", "Success".bright_green(), &cli.output);
    }

    Ok(())
}
