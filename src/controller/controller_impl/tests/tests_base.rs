use std::sync::Arc;
use tokio::sync::{oneshot, Mutex};

use super::*;
use crate::structs::*;
use crate::test::GitHubServer;

#[tokio::test]
async fn test_has_control_over() {
    let server = GitHubServer::new()
        .await
        .with_default_github_app()
        .with_default_app_installations();

    let c = new_controller(&server, true).await;

    assert!(c.has_control_over(&Actor {
        id: 1,
        login: "test-app[bot]".to_string()
    }));

    assert!(!c.has_control_over(&Actor {
        id: 1,
        login: "test-app".to_string()
    }));
    assert!(!c.has_control_over(&Actor {
        id: 1,
        login: "test-app[bot]extra".to_string()
    }));
    assert!(!c.has_control_over(&Actor {
        id: 2,
        login: "ppy".to_string()
    }));
}

#[tokio::test]
async fn test_has_control_over_uninitialized() {
    let server = GitHubServer::new()
        .await
        .with_default_github_app()
        .with_default_app_installations();

    let c = new_controller(&server, false).await;
    assert!(!c.has_control_over(&Actor {
        id: 1,
        login: "test-app[bot]".to_string()
    }));
    assert!(!c.has_control_over(&Actor {
        id: 2,
        login: "ppy".to_string()
    }));
}

#[tokio::test]
async fn test_run_forever_stops_after_transmitter_is_destroyed() {
    let server = GitHubServer::new()
        .await
        .with_default_github_app()
        .with_default_app_installations();

    let (request_tx, mut c) = make_controller(&server, false).await;
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
    let server = GitHubServer::new()
        .await
        .with_default_github_app()
        .with_default_app_installations();

    let (request_tx, c) = make_controller(&server, false).await;
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
async fn test_handle_message_pull_request_created() {
    let mut server = GitHubServer::new()
        .await
        .with_default_github_app()
        .with_default_app_installations();

    let pr = server.make_pull("test/repo", &["wiki/Article/en.md"]);
    server = server.with_pull("test/repo", &pr);

    let (request_tx, c) = make_controller(&server, true).await;
    let ctrl = Arc::new(Mutex::new(c));

    let _ = request_tx
        .send(ControllerRequest::PullRequestCreated {
            full_repo_name: "test/repo".into(),
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

    assert!(ctrl.lock().await.memory.pulls("test/repo").is_some());
}

#[tokio::test]
async fn test_handle_message_pull_request_created_and_updated() {
    let mut server = GitHubServer::new()
        .await
        .with_default_github_app()
        .with_default_app_installations();

    let pr = server.make_pull("test/repo", &["wiki/Article/en.md"]);
    server = server.with_pull("test/repo", &pr);

    let (request_tx, c) = make_controller(&server, true).await;
    let ctrl = Arc::new(Mutex::new(c));

    let _ = request_tx
        .send(ControllerRequest::PullRequestCreated {
            full_repo_name: "test/repo".into(),
            pull_request: Box::new(pr.clone()),
            trigger_updates: false,
        })
        .await;
    let _ = request_tx
        .send(ControllerRequest::PullRequestUpdated {
            full_repo_name: "test/repo".into(),
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
        ctrl.lock().await.memory.pulls("test/repo").unwrap().len(),
        1
    );
}

#[tokio::test]
async fn test_handle_message_pull_request_closed() {
    let mut server = GitHubServer::new()
        .await
        .with_default_github_app()
        .with_default_app_installations();

    let pr = server.make_pull("test/repo", &["wiki/Article/en.md"]);
    server = server.with_pull("test/repo", &pr);

    let (request_tx, c) = make_controller(&server, true).await;
    let ctrl = Arc::new(Mutex::new(c));

    let _ = request_tx
        .send(ControllerRequest::PullRequestCreated {
            full_repo_name: "test/repo".into(),
            pull_request: Box::new(pr.clone()),
            trigger_updates: false,
        })
        .await;
    let _ = request_tx
        .send(ControllerRequest::PullRequestClosed {
            full_repo_name: "test/repo".into(),
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
        .pulls("test/repo")
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn test_handle_message_installation_created() {
    let mut server = GitHubServer::new().await.with_default_github_app();

    let inst = server.make_installation();
    let inst_id = inst.id;
    let repo = server.make_repo(inst_id, "test/repo");
    server = server.with_app_installations(&[(inst.clone(), vec![repo])]);

    let (request_tx, c) = make_controller(&server, true).await;
    let ctrl = Arc::new(Mutex::new(c));

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

    assert_eq!(
        ctrl.lock().await.github.cached_repositories(inst_id).len(),
        1
    );
}

#[tokio::test]
async fn test_handle_message_installation_deleted() {
    let mut server = GitHubServer::new().await.with_default_github_app();

    let inst = server.make_installation();
    let inst_id = inst.id;
    let repo = server.make_repo(inst_id, "test/repo");
    server = server.with_app_installations(&[(inst.clone(), vec![repo])]);

    let (request_tx, c) = make_controller(&server, true).await;
    let ctrl = Arc::new(Mutex::new(c));

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

    assert!(ctrl
        .lock()
        .await
        .github
        .cached_repositories(inst_id)
        .is_empty());
}

#[tokio::test]
async fn test_handle_message_installation_repositories_added() {
    let mut server = GitHubServer::new().await.with_default_github_app();

    let inst = server.make_installation();
    let inst_id = inst.id;
    let repos = vec![
        server.make_repo(inst_id, "test/repo-1"),
        server.make_repo(inst_id, "test/repo-2"),
    ];
    server = server.with_app_installations(&[(inst.clone(), repos.clone())]);

    let (request_tx, c) = make_controller(&server, true).await;
    let ctrl = Arc::new(Mutex::new(c));

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
    let mut server = GitHubServer::new().await.with_default_github_app();

    let inst = server.make_installation();
    let inst_id = inst.id;
    let repos = vec![
        server.make_repo(inst_id, "test/repo-1"),
        server.make_repo(inst_id, "test/repo-2"),
    ];
    server = server.with_app_installations(&[(inst.clone(), repos.clone())]);

    let (request_tx, c) = make_controller(&server, true).await;
    let ctrl = Arc::new(Mutex::new(c));

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

#[tokio::test]
async fn test_reconcile_removes_closed_pull() {
    let mut server = GitHubServer::new().await.with_default_github_app();

    let inst = server.make_installation();
    let inst_id = inst.id;
    let repo = server.make_repo(inst_id, "test/repo");

    let pr1 = server.make_pull("test/repo", &["wiki/Article/en.md"]);
    let pr2 = server.make_pull("test/repo", &["wiki/Other/en.md"]);
    server = server
        .with_app_installations(&[(inst, vec![repo])])
        .with_pulls("test/repo", &[pr1.clone(), pr2.clone()]);

    let (request_tx, c) = make_controller(&server, true).await;
    server
        .server
        .mock(
            "GET",
            "/repos/test/repo/pulls?state=open&direction=asc&sort=created&per_page=100&page=1",
        )
        .with_status(200)
        .with_body(serde_json::to_string(&[pr2.clone()]).unwrap())
        .create();

    let ctrl = Arc::new(Mutex::new(c));
    let _ = request_tx.send(ControllerRequest::Reconcile).await;

    drop(request_tx);
    let cloned = ctrl.clone();
    tokio::join!(tokio::spawn(async move {
        cloned.lock().await.run_forever().await;
    }))
    .0
    .unwrap();

    let pulls = ctrl.lock().await.memory.pulls("test/repo").unwrap();
    assert!(
        pulls.contains_key(&pr2.number),
        "pr2 should remain in cache"
    );
    assert!(
        !pulls.contains_key(&pr1.number),
        "pr1 should be removed: it is no longer open on GitHub"
    );
}
