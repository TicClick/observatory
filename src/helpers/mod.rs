pub mod comments;
pub mod pulls;

pub trait ToMarkdown {
    fn to_markdown(&self) -> String;
}
