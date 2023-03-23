use super::{Controller, ControllerRequest};
use crate::test;

async fn make_controller(
    init: bool,
) -> (
    tokio::sync::mpsc::Sender<ControllerRequest>,
    Controller<test::DummyGitHubClient>,
) {
    let (tx, rx) = tokio::sync::mpsc::channel(10);
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
    (tx, c)
}

async fn new_controller(init: bool) -> Controller<test::DummyGitHubClient> {
    make_controller(init).await.1
}

mod tests_base;
mod tests_comments;
mod tests_conflicts;
mod tests_installations_repos;
