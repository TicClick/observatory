use super::*;

#[allow(unused_assignments)]
#[tokio::test]
async fn test_add_installations() {
    let mut server = GitHubServer::new()
        .with_default_github_app()
        .with_app_installations(&Vec::new());

    let c = new_controller(&server, true).await;

    let installations: Vec<_> = (0..10).map(|_| server.make_installation()).collect();
    let payload: Vec<_> = installations
        .iter()
        .map(|inst| (inst.clone(), Vec::new()))
        .collect();
    server = server.with_app_installations(&payload);

    for inst in &installations {
        c.add_installation(inst.clone()).await.unwrap();
    }
    let mut v = c.github.read_installations().await.unwrap();
    v.sort_by_key(|i| i.id);
    assert_eq!(v, installations);
}

#[allow(unused_assignments)]
#[tokio::test]
async fn test_add_installation_repositories_fetched() {
    let mut server = GitHubServer::new()
        .with_default_github_app()
        .with_app_installations(&Vec::new());

    let c = new_controller(&server, true).await;

    let installation = server.make_installation();
    let repos = vec![
        server.make_repo(installation.id, "test/repo"),
        server.make_repo(installation.id, "test/another-repo"),
    ];
    server = server.with_app_installations(&[(installation.clone(), repos.clone())]);

    c.add_installation(installation).await.unwrap();
    let mut fetched_repos = c.github.cached_repositories(1);
    fetched_repos.sort_by_key(|r| r.id);
    assert_eq!(repos, fetched_repos);
}

#[allow(unused_assignments)]
#[tokio::test]
async fn test_add_installation_pull_requests_fetched() {
    let mut server = GitHubServer::new()
        .with_default_github_app()
        .with_app_installations(&Vec::new());

    let c = new_controller(&server, true).await;

    let installation = server.make_installation();
    let repo = server.make_repo(installation.id, "test/my-repo");

    let pulls = [
        server.make_pull("test/my-repo", &["wiki/Article/en.md"]),
        server.make_pull("test/my-repo", &["wiki/Article/ko.md"]),
    ];
    server = server
        .with_app_installations(&[(installation.clone(), vec![repo])])
        .with_pulls("test/my-repo", &pulls);

    c.add_installation(installation.clone()).await.unwrap();
    let mut pulls_from_memory: Vec<_> = c
        .memory
        .pulls("test/my-repo")
        .unwrap()
        .keys()
        .cloned()
        .collect();
    pulls_from_memory.sort();

    assert_eq!(pulls_from_memory, vec![pulls[0].number, pulls[1].number]);
}

#[tokio::test]
async fn test_add_multiple_repositories_pull_requests_fetched() {
    let mut server = GitHubServer::new().with_default_github_app();

    let installation = server.make_installation();
    let repos = [
        server.make_repo(installation.id, "test/my-repo"),
        server.make_repo(installation.id, "test/my-other-repo"),
    ];

    let pulls = [
        server.make_pull("test/my-repo", &["wiki/Article/en.md"]),
        server.make_pull("test/my-repo", &["wiki/Article/ko.md"]),
        server.make_pull("test/my-other-repo", &["wiki/Other/ko.md"]),
    ];
    server = server
        .with_app_installations(&[(installation.clone(), repos.to_vec())])
        .with_pulls("test/my-repo", &pulls[0..2])
        .with_pull("test/my-other-repo", &pulls[2]);

    let c = new_controller(&server, true).await;
    c.add_repositories(1, repos.to_vec()).await;

    let mut first_batch: Vec<_> = c
        .memory
        .pulls("test/my-repo")
        .unwrap()
        .keys()
        .cloned()
        .collect();
    first_batch.sort();
    assert_eq!(first_batch, vec![pulls[0].number, pulls[1].number]);

    let mut second_batch: Vec<_> = c
        .memory
        .pulls("test/my-other-repo")
        .unwrap()
        .keys()
        .cloned()
        .collect();
    second_batch.sort();
    assert_eq!(second_batch, vec![pulls[2].number]);
}

#[tokio::test]
async fn test_repositories_fetched_during_init() {
    let mut server = GitHubServer::new().with_default_github_app();

    let installation = server.make_installation();
    let repos = [
        server.make_repo(installation.id, "test/my-repo"),
        server.make_repo(installation.id, "test/my-other-repo"),
    ];
    server = server.with_app_installations(&[(installation.clone(), repos.to_vec())]);

    let mut c = new_controller(&server, false).await;

    c.init().await.unwrap();
    let mut fetched_repos = c.github.cached_repositories(installation.id);
    fetched_repos.sort_by_key(|r| r.id);
    assert_eq!(fetched_repos, repos);
}

#[allow(unused_assignments)]
#[tokio::test]
async fn test_pulls_fetched_during_init() {
    let mut server = GitHubServer::new().with_default_github_app();

    let installation = server.make_installation();
    let repos = [
        server.make_repo(installation.id, "test/my-repo"),
        server.make_repo(installation.id, "test/other-repo-repo"),
    ];

    let mut c = new_controller(&server, false).await;

    let pulls = [
        server.make_pull("test/my-repo", &["wiki/Article/en.md"]),
        server.make_pull("test/my-repo", &["wiki/Article/ko.md"]),
        server.make_pull("test/other-repo-repo", &["wiki/Other/ko.md"]),
    ];

    server = server
        .with_app_installations(&[(installation.clone(), repos.to_vec())])
        .with_pulls("test/my-repo", &pulls[0..2])
        .with_pull("test/other-repo-repo", &pulls[2]);

    c.init().await.unwrap();

    let mut first_batch: Vec<_> = c
        .memory
        .pulls("test/my-repo")
        .unwrap()
        .keys()
        .cloned()
        .collect();
    first_batch.sort();
    assert_eq!(first_batch, vec![pulls[0].number, pulls[1].number]);

    let mut second_batch: Vec<_> = c
        .memory
        .pulls("test/other-repo-repo")
        .unwrap()
        .keys()
        .cloned()
        .collect();
    second_batch.sort();
    assert_eq!(second_batch, vec![pulls[2].number]);
}

#[allow(unused_assignments)]
#[tokio::test]
async fn test_delete_installation() {
    let mut server = GitHubServer::new().with_default_github_app();

    let installation = server.make_installation();
    let repos = [
        server.make_repo(installation.id, "test/my-repo"),
        server.make_repo(installation.id, "test/other-repo-repo"),
    ];

    let mut c = new_controller(&server, false).await;

    let pulls = [
        server.make_pull("test/my-repo", &["wiki/Article/en.md"]),
        server.make_pull("test/my-repo", &["wiki/Article/ko.md"]),
        server.make_pull("test/other-repo-repo", &["wiki/Other/en.md"]),
    ];

    server = server
        .with_app_installations(&[(installation.clone(), repos.to_vec())])
        .with_pull("test/my-repo", &pulls[0])
        .with_pull("test/my-repo", &pulls[1])
        .with_pull("test/other-repo-repo", &pulls[2]);

    c.init().await.unwrap();
    c.delete_installation(installation);

    assert!(c.memory.pulls.lock().unwrap().is_empty());
}

#[allow(unused_assignments)]
#[tokio::test]
async fn test_remove_repositories() {
    let mut server = GitHubServer::new().with_default_github_app();

    let installation = server.make_installation();
    let repos = [
        server.make_repo(installation.id, "test/my-repo"),
        server.make_repo(installation.id, "test/other-repo-repo"),
    ];

    let mut c = new_controller(&server, false).await;

    let pulls = [
        server.make_pull("test/my-repo", &["wiki/Article/en.md"]),
        server.make_pull("test/my-repo", &["wiki/Article/ko.md"]),
        server.make_pull("test/other-repo-repo", &["wiki/Other/ko.md"]),
    ];

    server = server
        .with_app_installations(&[(installation.clone(), repos.to_vec())])
        .with_pull("test/my-repo", &pulls[0])
        .with_pull("test/my-repo", &pulls[1])
        .with_pull("test/other-repo-repo", &pulls[2]);

    c.init().await.unwrap();
    c.remove_repositories(installation.id, &[repos[0].clone()]);

    let second_repo_only = c.memory.pulls.lock().unwrap();
    assert_eq!(second_repo_only.len(), 1);
    let cached_pr = second_repo_only
        .get("test/other-repo-repo")
        .unwrap()
        .values()
        .next()
        .take()
        .unwrap();
    assert_eq!(cached_pr.id, pulls[2].id);
}

#[allow(unused_assignments)]
#[tokio::test]
async fn test_remove_pull() {
    let mut server = GitHubServer::new().with_default_github_app();

    let installation = server.make_installation();
    let repos = [server.make_repo(installation.id, "test/my-repo")];

    let mut c = new_controller(&server, false).await;

    let pulls = [
        server.make_pull("test/my-repo", &["wiki/Article/en.md"]),
        server.make_pull("test/my-repo", &["wiki/Article/ko.md"]),
    ];

    server = server
        .with_app_installations(&[(installation.clone(), repos.to_vec())])
        .with_pulls("test/my-repo", &pulls);

    c.init().await.unwrap();
    c.finalize_pull("test/my-repo", pulls[1].clone()).await;

    let repos = c.memory.pulls.lock().unwrap();
    let first_repo = repos.get("test/my-repo").unwrap();
    let cached_pr = first_repo.values().next().take().unwrap();
    assert_eq!(cached_pr.id, 1);
}
