/// `pulls` contains structures and helpers for detecting conflicts between two pull requests.
use std::cmp::{PartialEq, PartialOrd};

use serde::{Deserialize, Serialize};

use crate::structs;

/// Types of pull conflicts
#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Hash, Serialize, Deserialize)]
pub enum ConflictType {
    /// Two pull requests have common file(s).
    /// Target = new pull, reference = old pull.
    ExistingChange,

    /// A new pull request affects an article for which there's a translation open.
    /// Target = old pull (translation), reference = new pull (original).
    NewOriginalChange,

    /// There is a new translation of the article that has a pending change.
    /// Target = new pull (translation), reference = old pull (original).
    ExistingOriginalChange,
}

/// A structure containing information about a conflict between two pull requests.
#[derive(Debug, Ord, Eq, PartialEq, PartialOrd, Clone)]
pub struct Conflict {
    /// Type of conflict.
    pub kind: ConflictType,

    /// The pull request which triggered the conflict and will be notified.
    /// Typically its author will need to follow the referenced pull for changes, and resolve conflicts.
    pub notification_target: i32,

    /// The pull request which is considered original. It is assumed to have higher priority (the other party will need to adjust).
    pub reference_target: i32,

    /// A GitHub URL to the "original" pull request.
    pub reference_url: String,

    /// List of conflicting files. May contain both translations and originals, but articles (= directories) are guaranteed to be unique.
    pub file_set: Vec<String>,
}

/// A lightweight article wrapper, made for ease of file path comparison.
pub struct Article {
    pub path: String,
    pub language: String,
}

impl Article {
    pub fn from_file_path(s: &str) -> Self {
        let fp = std::path::Path::new(s);
        let language = fp.file_stem().unwrap().to_str().unwrap().to_owned();
        let path = fp.parent().unwrap().to_str().unwrap().to_owned();
        Self { path, language }
    }

    pub fn file_path(&self) -> String {
        format!("{}/{}.md", self.path, self.language)
    }

    pub fn is_original(&self) -> bool {
        self.language == "en"
    }

    pub fn is_translation(&self) -> bool {
        !self.is_original()
    }
}

impl std::cmp::PartialEq for Article {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path && self.language == other.language
    }
}

/// Compare two pulls and pinpoint different types of conflicts between them on article level.
pub fn compare_pulls(
    new_pull: &structs::PullRequest,
    other_pull: &structs::PullRequest,
) -> Vec<Conflict> {
    let new_diff = new_pull.diff.as_ref().unwrap();
    let other_diff = other_pull.diff.as_ref().unwrap();

    let mut overlaps = Vec::new();
    let mut originals = Vec::new();
    let mut translations = Vec::new();

    for incoming in new_diff
        .files()
        .iter()
        .filter(|fp| fp.target_file.ends_with(".md"))
    {
        for other in other_diff
            .files()
            .iter()
            .filter(|fp| fp.target_file.ends_with(".md"))
        {
            let new_article = Article::from_file_path(&incoming.path());
            let other_article = Article::from_file_path(&other.path());

            // Different folders.
            if new_article.path != other_article.path {
                continue;
            }

            if new_article == other_article {
                overlaps.push(new_article.file_path());
                continue;
            }

            if new_article.is_original() && other_article.is_translation() {
                originals.push(new_article.file_path());
            } else if new_article.is_translation() && other_article.is_original() {
                translations.push(new_article.file_path());
            }
        }
    }

    overlaps.sort();
    originals.sort();
    translations.sort();

    let mut out = Vec::new();
    if !overlaps.is_empty() {
        out.push(Conflict {
            kind: ConflictType::ExistingChange,
            notification_target: new_pull.number,
            reference_target: other_pull.number,
            reference_url: other_pull.html_url.clone(),
            file_set: overlaps,
        });
    }
    if !originals.is_empty() {
        out.push(Conflict {
            kind: ConflictType::NewOriginalChange,
            notification_target: other_pull.number,
            reference_target: new_pull.number,
            reference_url: new_pull.html_url.clone(),
            file_set: originals,
        })
    }
    if !translations.is_empty() {
        out.push(Conflict {
            kind: ConflictType::ExistingOriginalChange,
            notification_target: new_pull.number,
            reference_target: other_pull.number,
            reference_url: other_pull.html_url.clone(),
            file_set: translations,
        })
    }
    out.sort();
    out
}
