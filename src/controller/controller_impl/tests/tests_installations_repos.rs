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
            repositories: Vec::new(),
        };
        c.add_installation(inst.clone()).await.unwrap();
        installations.push(inst);
    }
    let mut v = c.github.installations().await.unwrap();
    v.sort_by_key(|i| i.id);
    assert_eq!(v, installations);

    let mut vv = c.installations();
    vv.sort_by_key(|i| i.id);
    assert_eq!(vv, installations);
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
        repositories: Vec::new(),
    };

    c.add_installation(inst.clone()).await.unwrap();
    let mut v = c.installations();
    v[0].repositories.sort_by_key(|r| r.id);
    assert_eq!(v[0].repositories, vec![r1, r2]);
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
        repositories: Vec::new(),
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
    let repos = [
        c.github.test_add_repository(inst.id, "test/my-repo"),
        c.github
            .test_add_repository(inst.id, "test/other-repo-repo"),
    ];

    c.init().await.unwrap();
    let mut v = c.installations();
    v[0].repositories.sort_by_key(|r| r.id);
    assert_eq!(v[0].repositories, repos);
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

