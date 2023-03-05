pub mod comments;
pub mod conflicts;

pub trait ToMarkdown {
    fn to_markdown(&self) -> String;
}
