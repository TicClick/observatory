use super::*;
use crate::github::GitHubInterface;
use crate::structs::*;

#[tokio::test]
async fn test_add_installations() {
    let c = make_controller(true).await;
    let mut installations = vec![];
    for i in 0..10 {
        let inst = Installation {
            id: i,
            account: Actor {
                id: 12,
                login: "test-user".into(),
            },
            app_id: 123,
        };
        c.add_installation(inst.clone()).await.unwrap();
        installations.push(inst);
    }
    let mut v = c.github.installations().await.unwrap();
    v.sort_by_key(|i| i.id);
    assert_eq!(v, installations);
}

#[tokio::test]
async fn test_add_installation_repositories_fetched() {
    let c = make_controller(true).await;
    let r1 = c.github.test_add_repository(1, "test/repo");
    let r2 = c.github.test_add_repository(1, "test/another-repo");

    let inst = Installation {
        id: 1,
        account: Actor {
            id: 12,
            login: "test-user".into(),
        },
        app_id: 123,
    };

    c.add_installation(inst).await.unwrap();
    let mut repos = c.github.cached_repositories(1);
    repos.sort_by_key(|r| r.id);
    assert_eq!(repos, vec![r1, r2]);
}

#[tokio::test]
async fn test_add_installation_pull_requests_fetched() {
    let c = make_controller(true).await;

    c.github.test_add_repository(1, "test/my-repo");
    let pulls = [
        c.github
            .test_add_pull("test/my-repo", &["wiki/Article/en.md"]),
        c.github
            .test_add_pull("test/my-repo", &["wiki/Article/ko.md"]),
    ];

    let inst = Installation {
        id: 1,
        account: Actor {
            id: 12,
            login: "test-user".into(),
        },
        app_id: 123,
    };

    c.add_installation(inst.clone()).await.unwrap();
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
async fn test_add_repository_pull_requests_fetched() {
    let c = make_controller(true).await;

    let repo = c.github.test_add_repository(1, "test/my-repo");
    let pulls = [
        c.github
            .test_add_pull("test/my-repo", &["wiki/Article/en.md"]),
        c.github
            .test_add_pull("test/my-repo", &["wiki/Article/ko.md"]),
    ];

    c.add_repository(&repo).await.unwrap();
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
    let c = make_controller(true).await;

    let repos = [
        c.github.test_add_repository(1, "test/my-repo"),
        c.github.test_add_repository(1, "test/other-repo-repo"),
    ];

    let pulls = [
        c.github
            .test_add_pull("test/my-repo", &["wiki/Article/en.md"]),
        c.github
            .test_add_pull("test/my-repo", &["wiki/Article/ko.md"]),
        c.github
            .test_add_pull("test/other-repo-repo", &["wiki/Other/ko.md"]),
    ];

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
        .pulls("test/other-repo-repo")
        .unwrap()
        .keys()
        .cloned()
        .collect();
    second_batch.sort();
    assert_eq!(second_batch, vec![pulls[2].number]);
}

#[tokio::test]
async fn test_repositories_fetched_during_init() {
    let mut c = make_controller(false).await;

    let inst = c.github.test_add_installation();
    let repositories = [
        c.github.test_add_repository(inst.id, "test/my-repo"),
        c.github
            .test_add_repository(inst.id, "test/other-repo-repo"),
    ];

    c.init().await.unwrap();
    let mut repos = c.github.cached_repositories(inst.id);
    repos.sort_by_key(|r| r.id);
    assert_eq!(repos, repositories);
}

#[tokio::test]
async fn test_pulls_fetched_during_init() {
    let mut c = make_controller(false).await;

    let inst = c.github.test_add_installation();
    c.github.test_add_repository(inst.id, "test/my-repo");
    c.github
        .test_add_repository(inst.id, "test/other-repo-repo");

    let pulls = [
        c.github
            .test_add_pull("test/my-repo", &["wiki/Article/en.md"]),
        c.github
            .test_add_pull("test/my-repo", &["wiki/Article/ko.md"]),
        c.github
            .test_add_pull("test/other-repo-repo", &["wiki/Other/ko.md"]),
    ];

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

#[tokio::test]
async fn test_delete_installation() {
    let mut c = make_controller(false).await;

    let inst = c.github.test_add_installation();
    c.github.test_add_repository(inst.id, "test/my-repo");
    c.github
        .test_add_repository(inst.id, "test/other-repo-repo");

    c.github
        .test_add_pull("test/my-repo", &["wiki/Article/en.md"]);
    c.github
        .test_add_pull("test/my-repo", &["wiki/Article/ko.md"]);
    c.github
        .test_add_pull("test/other-repo-repo", &["wiki/Other/ko.md"]);

    c.init().await.unwrap();
    c.delete_installation(inst);

    assert!(c.github.cached_installations().is_empty());

    assert!(c.memory.pulls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn test_remove_repositories() {
    let mut c = make_controller(false).await;

    let inst = c.github.test_add_installation();
    let r1 = c.github.test_add_repository(inst.id, "test/my-repo");
    c.github
        .test_add_repository(inst.id, "test/other-repo-repo");

    c.github
        .test_add_pull("test/my-repo", &["wiki/Article/en.md"]);
    c.github
        .test_add_pull("test/my-repo", &["wiki/Article/ko.md"]);
    let preserved_pr = c
        .github
        .test_add_pull("test/other-repo-repo", &["wiki/Other/ko.md"]);

    c.init().await.unwrap();
    c.remove_repositories(inst.id, &[r1]);

    assert!(!c.github.cached_installations().is_empty());

    let second_repo_only = c.memory.pulls.lock().unwrap();
    assert_eq!(second_repo_only.len(), 1);
    let cached_pr = second_repo_only
        .get("test/other-repo-repo")
        .unwrap()
        .values()
        .next()
        .take()
        .unwrap();
    assert_eq!(cached_pr.id, preserved_pr.id);
}

#[tokio::test]
async fn test_remove_pull() {
    let mut c = make_controller(false).await;

    let inst = c.github.test_add_installation();
    c.github.test_add_repository(inst.id, "test/my-repo");
    c.github
        .test_add_pull("test/my-repo", &["wiki/Article/en.md"]);
    let pull_to_remove = c
        .github
        .test_add_pull("test/my-repo", &["wiki/Article/ko.md"]);

    c.init().await.unwrap();
    c.remove_pull("test/my-repo", pull_to_remove);

    let repos = c.memory.pulls.lock().unwrap();
    let first_repo = repos.get("test/my-repo").unwrap();
    let cached_pr = first_repo.values().next().take().unwrap();
    assert_eq!(cached_pr.id, 1);
}
