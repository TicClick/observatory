use super::Controller;
use crate::test;

async fn make_controller(init: bool) -> Controller<test::DummyGitHubClient> {
    let (_, rx) = tokio::sync::mpsc::channel(10);
    let mut c = Controller::<_>::new(
        rx,
        "123".to_string(),
        "private-key".to_string(),
        crate::config::Controller {
            post_comments: true,
        },
    );
    if init {
        c.init().await.unwrap();
    }
    c
}

mod tests_base;
mod tests_comments;
mod tests_conflicts;
mod tests_installations_repos;
