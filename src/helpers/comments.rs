/// `comments` contains a helper for converting conflicts into YAML headers for GitHub comments,
/// as well as comment templates.
use serde::{Deserialize, Serialize};

use crate::helpers::pulls::ConflictType;
use crate::helpers::ToMarkdown;

/// Warn the author of a new pull request about someone else's unmerged work.
pub const EXISTING_CHANGE_TEMPLATE: &str = "Someone else has edited same files as you did. Please check their changes in case they conflict with yours:\n";

/// Warn the author of an existing translation request about new changes in the original article.
pub const NEW_ORIGINAL_CHANGE_TEMPLATE: &str = "Some articles might have changes that are missing from your translation. Please update it after they are merged:\n";

/// Warn the author of a new translation request about existing changes in the original article.
pub const EXISTING_ORIGINAL_CHANGE_TEMPLATE: &str = NEW_ORIGINAL_CHANGE_TEMPLATE;

pub const HTML_COMMENT_START: &str = "<!--";
pub const HTML_COMMENT_END: &str = "-->";

/// Structured header for comments made by the bot, designed to avoid tedious and error-prone parsing.
#[derive(Debug, Serialize, Deserialize, PartialEq, PartialOrd, Ord, Eq)]
pub struct CommentHeader {
    pub pull_number: i32,
    pub conflict_type: ConflictType,
}

impl CommentHeader {
    /// Attempt to extract the header from a Markdown comment.
    /// The header is expected to look like this, with HTML comment tags on separate lines:
    /// ```ignore
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

#[cfg(test)]
#[path = "comments_test.rs"]
pub(crate) mod tests;
