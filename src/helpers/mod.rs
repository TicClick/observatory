pub mod cgroup;
pub mod comments;
pub mod conflicts;
pub mod digest;

pub trait ToMarkdown {
    fn to_markdown(&self) -> String;
}
