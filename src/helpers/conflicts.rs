/// `pulls` contains structures and helpers for detecting conflicts between two pull requests.
use std::cmp::{PartialEq, PartialOrd};
use std::collections::hash_map::Entry;
use std::collections::{BTreeSet, HashMap};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

use crate::helpers::ToMarkdown;
use crate::helpers::{comments, digest};
use crate::structs;

/// Types of pull conflicts
#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Hash, Serialize, Deserialize)]
pub enum ConflictType {
    /// Two pull requests have common file(s).
    /// Target = new pull, reference = old pull.
    Overlap,

    /// A merged pull request affects an article for which there's a translation open.
    /// Target = old pull (translation), reference = merged pull (original).
    IncompleteTranslation,
}

impl ToMarkdown for ConflictType {
    fn to_markdown(&self) -> String {
        match self {
            ConflictType::Overlap => comments::OVERLAP_TEMPLATE,
            ConflictType::IncompleteTranslation => comments::INCOMPLETE_TRANSLATION_TEMPLATE,
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
    pub fn overlap(
        trigger: i32,
        original: i32,
        reference_url: String,
        file_set: Vec<String>,
    ) -> Self {
        Self {
            kind: ConflictType::Overlap,
            trigger,
            original,
            reference_url,
            file_set,
        }
    }
    pub fn incomplete_translation(
        trigger: i32,
        original: i32,
        reference_url: String,
        file_set: Vec<String>,
    ) -> Self {
        Self {
            kind: ConflictType::IncompleteTranslation,
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
            for file in &self.file_set {
                let file_name_hash = digest::hash_data(&ring::digest::SHA256, file.as_bytes());
                let file_link = format!("{}/files#diff-{}", self.reference_url, file_name_hash);
                lines.push(format!("{indent}- [`{file}`]({file_link})"));
            }
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

    pub fn original_file_path(&self) -> String {
        format!("{}/en.md", self.path)
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

    let mut overlaps = BTreeSet::new();
    let mut originals = BTreeSet::new();

    let mut is_new_translation = false;

    // Only consider Markdown files, and among these, exclude non-articles such as the tournament template
    // (https://github.com/TicClick/observatory/issues/17)
    let article_expr = regex::Regex::new(r"^(..|..-..)\.md$").unwrap();
    let article_filter = |p: &&unidiff::PatchedFile| -> bool {
        // For deleted files, target_file is /dev/null -- check source_file instead
        let file_to_check = match p.target_file.contains("/dev/null") {
            true => &p.source_file,
            false => &p.target_file,
        };
        let parts = std::path::Path::new(file_to_check);
        if let Some(filename) = parts.file_name() {
            if let Some(filename_cstr) = filename.to_str() {
                article_expr.is_match(filename_cstr)
            } else {
                false
            }
        } else {
            false
        }
    };

    for incoming in new_diff.files().iter().filter(article_filter) {
        let other_files: BTreeSet<_> = other_diff
            .files()
            .iter()
            .filter(article_filter)
            .map(|patched| patched.path())
            .collect();
        for other in other_files.iter() {
            let new_article = Article::from_file_path(&incoming.path());
            let other_article = Article::from_file_path(other);

            // Different folders.
            if new_article.path != other_article.path {
                continue;
            }

            // Protect against duplicate conflicts when an original change also marks translations as outdated:
            // [EN (meaningful update), RU (outdate translation)] vs [RU (translation update)] produces only one conflict (IncompleteTranslation).
            let translation_only_change = new_article.is_translation()
                && !other_files.contains(&new_article.original_file_path());

            if new_article == other_article
                && (new_article.is_original() || translation_only_change)
            {
                overlaps.insert(new_article.file_path());
                continue;
            }

            if new_article.is_original() && other_article.is_translation() {
                originals.insert(new_article.file_path());
            } else if other_article.is_original() && new_article.is_translation() {
                originals.insert(other_article.file_path());
                is_new_translation = true;
            }
        }
    }

    let mut out = Vec::new();
    if !overlaps.is_empty() {
        out.push(Conflict::overlap(
            new_pull.number,
            other_pull.number,
            other_pull.html_url.clone(),
            overlaps.into_iter().collect(),
        ));
    }

    if !originals.is_empty() {
        let (trigger, original) = if is_new_translation {
            (&new_pull, &other_pull)
        } else {
            (&other_pull, &new_pull)
        };
        out.push(Conflict::incomplete_translation(
            trigger.number,
            original.number,
            original.html_url.clone(),
            originals.into_iter().collect(),
        ));
    }
    out.sort();
    out
}

type ConflictKey = (i32, i32, ConflictType);
pub fn make_conflict_key(original: i32, trigger: i32, kind: &ConflictType) -> ConflictKey {
    if original < trigger {
        (original, trigger, kind.clone())
    } else {
        (trigger, original, kind.clone())
    }
}
impl Conflict {
    pub fn key(&self) -> ConflictKey {
        make_conflict_key(self.original, self.trigger, &self.kind)
    }
}

/// Helper for storing and updating conflicts between pull requests.
#[derive(Default, Debug, Clone)]
pub struct Storage {
    map: Arc<Mutex<HashMap<String, HashMap<ConflictKey, Conflict>>>>,
}

impl Storage {
    /// Record or update a conflict, and return its updated version, so that the controller
    /// can send notifications 1) to correct pull, and 2) with proper metadata. The latter is important because
    /// the controller doesn't have full information about what should be posted and where after the update.
    pub fn upsert(&self, full_repo_name: &str, c: &Conflict) -> Option<Conflict> {
        let mut all_conflicts = self.map.lock().unwrap();
        let repo_conflicts = all_conflicts.entry(full_repo_name.to_string()).or_default();
        match repo_conflicts.entry(c.key()) {
            Entry::Vacant(e) => {
                e.insert(c.clone());
                Some(c.clone())
            }
            Entry::Occupied(mut e) => {
                let existing_conflict = e.get_mut();
                if existing_conflict == c {
                    None
                } else {
                    existing_conflict.file_set = c.file_set.clone();
                    Some(existing_conflict.clone())
                }
            }
        }
    }

    /// Return only conflicts for which `predicate` returns `true`.
    fn select_conflicts<F>(&self, full_repo_name: &str, predicate: F) -> Vec<Conflict>
    where
        F: Fn(&Conflict) -> bool,
    {
        match self.map.lock().unwrap().get(full_repo_name) {
            None => Vec::new(),
            Some(m) => {
                let mut conflicts: Vec<_> = m.values().filter(|c| predicate(c)).cloned().collect();
                conflicts.sort();
                conflicts
            }
        }
    }

    /// Remove conflicts for which `predicate` returns `true`, preserving anything else.
    fn prune_conflicts<F>(&self, full_repo_name: &str, predicate: F)
    where
        F: Fn(&Conflict) -> bool,
    {
        if let Some(m) = self.map.lock().unwrap().get_mut(full_repo_name) {
            m.retain(|_, v| !predicate(v));
        }
    }

    /// Return conflicts with `pull_number` as the original pull, which counts as the source of truth.
    pub fn by_original(&self, full_repo_name: &str, pull_number: i32) -> Vec<Conflict> {
        self.select_conflicts(full_repo_name, |c| c.original == pull_number)
    }

    /// Return conflicts with `pull_number` as the trigger pull, which initiated the conflict and needs to be updated.
    pub fn by_trigger(&self, full_repo_name: &str, pull_number: i32) -> Vec<Conflict> {
        self.select_conflicts(full_repo_name, |c| c.trigger == pull_number)
    }

    /// Remove cached conflicts which are not present anymore (stale).
    pub fn remove_missing(
        &self,
        full_repo_name: &str,
        original: i32,
        trigger: i32,
        detected: &[Conflict],
    ) -> Vec<Conflict> {
        if let Some(m) = self.map.lock().unwrap().get_mut(full_repo_name) {
            let possible_keys = [
                make_conflict_key(original, trigger, &ConflictType::Overlap),
                make_conflict_key(original, trigger, &ConflictType::IncompleteTranslation),
            ];
            let keys_to_preserve: Vec<_> = detected.iter().map(|c| c.key()).collect();
            let mut removed = Vec::new();
            for k in possible_keys
                .iter()
                .filter(|kk| !keys_to_preserve.contains(kk))
            {
                if let Some(c) = m.remove(k) {
                    removed.push(c);
                }
            }
            return removed;
        }
        Vec::new()
    }

    /// Remove any conflicts involving `pull_number` being either the original or the cause of conflict.
    pub fn remove_conflicts_by_pull(&self, full_repo_name: &str, pull_number: i32) {
        self.prune_conflicts(full_repo_name, |c| {
            c.trigger == pull_number || c.original == pull_number
        });
    }

    pub fn remove_repository(&self, full_repo_name: &str) {
        self.map.lock().unwrap().remove(&full_repo_name.to_string());
    }
}

#[cfg(test)]
mod tests;
