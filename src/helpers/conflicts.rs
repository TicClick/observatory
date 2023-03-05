/// `pulls` contains structures and helpers for detecting conflicts between two pull requests.
use std::cmp::{PartialEq, PartialOrd};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use crate::helpers::comments;
use crate::helpers::ToMarkdown;
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

impl ToMarkdown for ConflictType {
    fn to_markdown(&self) -> String {
        match self {
            ConflictType::ExistingChange => comments::EXISTING_CHANGE_TEMPLATE,
            ConflictType::NewOriginalChange => comments::NEW_ORIGINAL_CHANGE_TEMPLATE,
            ConflictType::ExistingOriginalChange => comments::EXISTING_ORIGINAL_CHANGE_TEMPLATE,
        }
        .to_string()
    }
}

/// A structure containing information about a conflict between two pull requests.
#[derive(Debug, Ord, Eq, PartialEq, PartialOrd, Clone)]
pub struct Conflict {
    /// Type of conflict.
    pub kind: ConflictType,

    /// The pull request which triggered the conflict and will be notified.
    /// Typically its author will need to follow the referenced pull for changes, and resolve conflicts.
    pub trigger: i32,

    /// The pull request which is considered original. It is assumed to have higher priority (the other party will need to adjust).
    pub original: i32,

    /// A GitHub URL to the "original" pull request.
    pub reference_url: String,

    /// List of conflicting files. May contain both translations and originals, but articles (= directories) are guaranteed to be unique.
    pub file_set: Vec<String>,
}

impl Conflict {
    pub fn new(
        kind: ConflictType,
        trigger: i32,
        original: i32,
        reference_url: String,
        file_set: Vec<String>,
    ) -> Self {
        Self {
            kind,
            trigger,
            original,
            reference_url,
            file_set,
        }
    }
    pub fn existing_change(
        trigger: i32,
        original: i32,
        reference_url: String,
        file_set: Vec<String>,
    ) -> Self {
        Self {
            kind: ConflictType::ExistingChange,
            trigger,
            original,
            reference_url,
            file_set,
        }
    }
    pub fn new_original_change(
        trigger: i32,
        original: i32,
        reference_url: String,
        file_set: Vec<String>,
    ) -> Self {
        Self {
            kind: ConflictType::NewOriginalChange,
            trigger,
            original,
            reference_url,
            file_set,
        }
    }
    pub fn existing_original_change(
        trigger: i32,
        original: i32,
        reference_url: String,
        file_set: Vec<String>,
    ) -> Self {
        Self {
            kind: ConflictType::ExistingOriginalChange,
            trigger,
            original,
            reference_url,
            file_set,
        }
    }
}

impl ToMarkdown for Conflict {
    fn to_markdown(&self) -> String {
        let header = comments::CommentHeader {
            pull_number: self.original,
            conflict_type: self.kind.clone(),
        };
        let mut lines = Vec::new();
        lines.push(header.to_markdown());
        lines.push(self.kind.to_markdown());

        if self.file_set.len() > 10 {
            lines.push(format!("- {} (>10 files)", self.reference_url));
        } else {
            lines.push(format!("- {}, files:", self.reference_url));
            let indent = "  ";
            lines.push(format!("{indent}```"));
            for file in &self.file_set {
                lines.push(format!("{indent}{file}"));
            }
            lines.push(format!("{indent}```"));
        }

        lines.join("\n")
    }
}

/// A lightweight article wrapper, made for ease of file path comparison.
#[derive(Debug)]
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
        out.push(Conflict::existing_change(
            new_pull.number,
            other_pull.number,
            other_pull.html_url.clone(),
            overlaps,
        ));
    }
    if !originals.is_empty() {
        out.push(Conflict::new_original_change(
            other_pull.number,
            new_pull.number,
            new_pull.html_url.clone(),
            originals,
        ));
    }
    if !translations.is_empty() {
        out.push(Conflict::existing_original_change(
            new_pull.number,
            other_pull.number,
            other_pull.html_url.clone(),
            translations,
        ));
    }
    out.sort();
    out
}

type ConflictKey = (i32, i32, ConflictType);
impl Conflict {
    pub fn key(&self) -> ConflictKey {
        (
            std::cmp::min(self.original, self.trigger),
            std::cmp::max(self.original, self.trigger),
            self.kind.clone(),
        )
    }
}

#[derive(Default, Debug, Clone)]
pub struct Storage {
    map: Arc<Mutex<HashMap<String, HashMap<ConflictKey, Conflict>>>>,
}

impl Storage {
    pub fn upsert(&self, full_repo_name: &str, c: &Conflict) -> bool {
        let mut all_conflicts = self.map.lock().unwrap();
        let repo_conflicts = all_conflicts.entry(full_repo_name.to_string()).or_default();
        let entry = repo_conflicts.entry(c.key()).or_insert(c.clone());
        if entry == c {
            false
        } else {
            entry.file_set = c.file_set.clone();
            true
        }
    }

    fn select_conflicts<F>(&self, full_repo_name: &str, predicate: F) -> Vec<Conflict>
    where
        F: Fn(&Conflict) -> bool,
    {
        match self.map.lock().unwrap().get(full_repo_name) {
            None => Vec::new(),
            Some(m) => {
                let mut conflicts: Vec<_> = m
                    .values()
                    .filter(|c| predicate(c))
                    .map(|c| c.clone())
                    .collect();
                conflicts.sort();
                conflicts
            }
        }
    }

    pub fn by_original(&self, full_repo_name: &str, pull_number: i32) -> Vec<Conflict> {
        self.select_conflicts(full_repo_name, |c| c.original == pull_number)
    }

    pub fn by_trigger(&self, full_repo_name: &str, pull_number: i32) -> Vec<Conflict> {
        self.select_conflicts(full_repo_name, |c| c.trigger == pull_number)
    }
}

#[cfg(test)]
#[path = "conflicts_test.rs"]
pub(crate) mod conflicts_test;
