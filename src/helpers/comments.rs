/// `comments` contains helpers for converting structures into pretty pieces of text for GitHub comments,
/// as well as comment templates.
use serde::{Deserialize, Serialize};

use crate::helpers::pulls::{Conflict, ConflictType};

/// Warn the author of a new pull request about someone else's unmerged work.
const EXISTING_CHANGE_TEMPLATE: &str = "Someone else has edited same files as you did. Please check their changes in case they conflict with yours:\n";

/// Warn the author of an existing translation request about new changes in the original article.
const NEW_ORIGINAL_CHANGE_TEMPLATE: &str = "Some articles might have changes that are missing from your translation. Please update it after they are merged:\n";

/// Warn the author of a new translation request about existing changes in the original article.
const EXISTING_ORIGINAL_CHANGE_TEMPLATE: &str = NEW_ORIGINAL_CHANGE_TEMPLATE;

pub const HTML_COMMENT_START: &str = "<!--";
pub const HTML_COMMENT_END: &str = "-->";

pub trait ToMarkdown {
    fn to_markdown(&self) -> String;
}
impl ToMarkdown for ConflictType {
    fn to_markdown(&self) -> String {
        match self {
            ConflictType::ExistingChange => EXISTING_CHANGE_TEMPLATE,
            ConflictType::NewOriginalChange => NEW_ORIGINAL_CHANGE_TEMPLATE,
            ConflictType::ExistingOriginalChange => EXISTING_ORIGINAL_CHANGE_TEMPLATE,
        }
        .to_string()
    }
}

/// Structured header for comments made by the bot, designed to avoid tedious and error-prone parsing.
#[derive(Debug, Serialize, Deserialize)]
pub struct CommentHeader {
    pub pull_number: i32,
    pub conflict_type: ConflictType,
}

impl CommentHeader {
    /// Attempt to extract the header from a Markdown comment.
    /// The header is expected to look like this, with HTML comment tags on separate lines:
    /// ```
    /// <!--
    ///   key1: value1
    ///   key2: value2
    /// -->
    /// ```
    pub fn from_comment(body: &str) -> Option<Self> {
        if !body.starts_with(HTML_COMMENT_START) {
            return None;
        }
        let mut lines = Vec::new();
        for line in body.split('\n') {
            if line.starts_with(HTML_COMMENT_START) {
                continue;
            }
            if line.starts_with(HTML_COMMENT_END) {
                break;
            }
            lines.push(line.to_string());
        }
        serde_yaml::from_str(&lines.join("\n")).ok()
    }
}

impl ToMarkdown for CommentHeader {
    fn to_markdown(&self) -> String {
        format!(
            "{}\n{}\n{}",
            HTML_COMMENT_START,
            serde_yaml::to_string(&self).unwrap().trim(),
            HTML_COMMENT_END
        )
    }
}

impl ToMarkdown for Conflict {
    fn to_markdown(&self) -> String {
        let header = CommentHeader {
            pull_number: self.reference_target,
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
