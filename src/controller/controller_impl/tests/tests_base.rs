use std::sync::Arc;
use tokio::sync::{oneshot, Mutex};

use super::*;
use crate::{github::GitHubInterface, structs::*};

#[tokio::test]
async fn test_has_control_over() {
    let c = new_controller(true).await;

    assert!(c.has_control_over(&Actor {
        id: 1,
        login: "test-app[bot]".to_string()
    }));
    assert!(!c.has_control_over(&Actor {
        id: 1,
        login: "test-app".to_string()
    }));
    assert!(!c.has_control_over(&Actor {
        id: 2,
        login: "ppy".to_string()
    }));
}

#[tokio::test]
async fn test_run_forever_stops_after_transmitter_is_destroyed() {
    let (request_tx, mut c) = make_controller(false).await;
    let handle = async move {
        c.run_forever().await;
    };

    let (tx, rx) = oneshot::channel();
    let _ = request_tx
        .send(ControllerRequest::Init { reply_to: tx })
        .await;

    drop(request_tx);
    tokio::join!(handle);
    assert!(rx.await.is_ok());
}

#[tokio::test]
async fn test_handle_message_init() {
    let (request_tx, c) = make_controller(false).await;
    let ctrl = Arc::new(Mutex::new(c));

    let (tx, rx) = oneshot::channel();
    let _ = request_tx
        .send(ControllerRequest::Init { reply_to: tx })
        .await;

    drop(request_tx);
    let cloned = ctrl.clone();
    tokio::join!(tokio::spawn(async move {
        cloned.lock().await.run_forever().await;
    }))
    .0
    .unwrap();

    assert!(rx.await.is_ok());
    assert!(ctrl.lock().await.app.is_some());
}

#[tokio::test]
async fn test_handle_message_pull_request_updated() {
    let (request_tx, c) = make_controller(true).await;
    let ctrl = Arc::new(Mutex::new(c));

    let pr = ctrl
        .lock()
        .await
        .github
        .test_add_pull("test/repo-name", &["wiki/Article/en.md"]);
    let _ = request_tx
        .send(ControllerRequest::PullRequestUpdated {
            full_repo_name: "test/repo-name".into(),
            pull_request: Box::new(pr),
            trigger_updates: false,
        })
        .await;

    drop(request_tx);
    let cloned = ctrl.clone();
    tokio::join!(tokio::spawn(async move {
        cloned.lock().await.run_forever().await;
    }))
    .0
    .unwrap();

    assert_eq!(
        ctrl.lock()
            .await
            .memory
            .pulls("test/repo-name")
            .unwrap()
            .len(),
        1
    );
}

#[tokio::test]
async fn test_handle_message_pull_request_closed() {
    let (request_tx, c) = make_controller(true).await;
    let ctrl = Arc::new(Mutex::new(c));

    let pr = ctrl
        .lock()
        .await
        .github
        .test_add_pull("test/repo-name", &["wiki/Article/en.md"]);
    let _ = request_tx
        .send(ControllerRequest::PullRequestUpdated {
            full_repo_name: "test/repo-name".into(),
            pull_request: Box::new(pr.clone()),
            trigger_updates: false,
        })
        .await;
    let _ = request_tx
        .send(ControllerRequest::PullRequestClosed {
            full_repo_name: "test/repo-name".into(),
            pull_request: Box::new(pr),
        })
        .await;

    drop(request_tx);
    let cloned = ctrl.clone();
    tokio::join!(tokio::spawn(async move {
        cloned.lock().await.run_forever().await;
    }))
    .0
    .unwrap();

    assert!(ctrl
        .lock()
        .await
        .memory
        .pulls("test/repo-name")
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn test_handle_message_installation_created() {
    let (request_tx, c) = make_controller(true).await;
    let ctrl = Arc::new(Mutex::new(c));

    let inst = ctrl.lock().await.github.test_add_installation();
    let _ = request_tx
        .send(ControllerRequest::InstallationCreated {
            installation: Box::new(inst),
        })
        .await;

    drop(request_tx);
    let cloned = ctrl.clone();
    tokio::join!(tokio::spawn(async move {
        cloned.lock().await.run_forever().await;
    }))
    .0
    .unwrap();

    assert_eq!(ctrl.lock().await.github.cached_installations().len(), 1);
}

#[tokio::test]
async fn test_handle_message_installation_deleted() {
    let (request_tx, c) = make_controller(true).await;
    let ctrl = Arc::new(Mutex::new(c));

    let inst = ctrl.lock().await.github.test_add_installation();
    let _ = request_tx
        .send(ControllerRequest::InstallationCreated {
            installation: Box::new(inst.clone()),
        })
        .await;
    let _ = request_tx
        .send(ControllerRequest::InstallationDeleted {
            installation: Box::new(inst.clone()),
        })
        .await;

    drop(request_tx);
    let cloned = ctrl.clone();
    tokio::join!(tokio::spawn(async move {
        cloned.lock().await.run_forever().await;
    }))
    .0
    .unwrap();

    assert!(ctrl.lock().await.github.cached_installations().is_empty());
}

#[tokio::test]
async fn test_handle_message_installation_repositories_added() {
    let (request_tx, c) = make_controller(true).await;
    let ctrl = Arc::new(Mutex::new(c));

    let (inst, repos) = {
        let gh = &ctrl.lock().await.github;
        let inst = gh.test_add_installation();
        let repos = [
            gh.test_add_repository(inst.id, "test/repo"),
            gh.test_add_repository(inst.id, "test/repo-2"),
        ];
        (inst, repos.to_vec())
    };

    let _ = request_tx
        .send(ControllerRequest::InstallationCreated {
            installation: Box::new(inst.clone()),
        })
        .await;
    let _ = request_tx
        .send(ControllerRequest::InstallationRepositoriesAdded {
            installation_id: inst.id,
            repositories: repos,
        })
        .await;

    drop(request_tx);
    let cloned = ctrl.clone();
    tokio::join!(tokio::spawn(async move {
        cloned.lock().await.run_forever().await;
    }))
    .0
    .unwrap();

    assert_eq!(
        ctrl.lock().await.github.cached_repositories(inst.id).len(),
        2
    );
}

#[tokio::test]
async fn test_handle_message_installation_repositories_removed() {
    let (request_tx, c) = make_controller(true).await;
    let ctrl = Arc::new(Mutex::new(c));

    let (inst, repos) = {
        let gh = &ctrl.lock().await.github;
        let inst = gh.test_add_installation();
        let repos = [
            gh.test_add_repository(inst.id, "test/repo"),
            gh.test_add_repository(inst.id, "test/repo-2"),
        ];
        (inst, repos.to_vec())
    };

    let removed_repo = repos[0].clone();
    let retained_repo = repos[1].clone();

    let _ = request_tx
        .send(ControllerRequest::InstallationCreated {
            installation: Box::new(inst.clone()),
        })
        .await;
    let _ = request_tx
        .send(ControllerRequest::InstallationRepositoriesAdded {
            installation_id: inst.id,
            repositories: repos.clone(),
        })
        .await;
    let _ = request_tx
        .send(ControllerRequest::InstallationRepositoriesRemoved {
            installation_id: inst.id,
            repositories: vec![removed_repo],
        })
        .await;

    drop(request_tx);
    let cloned = ctrl.clone();
    tokio::join!(tokio::spawn(async move {
        cloned.lock().await.run_forever().await;
    }))
    .0
    .unwrap();

    assert_eq!(
        ctrl.lock().await.github.cached_repositories(inst.id),
        vec![retained_repo]
    );
}
