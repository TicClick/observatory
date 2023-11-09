use super::*;

use crate::helpers::conflicts::Conflict;

#[tokio::test]
async fn test_add_pull() {
    let mut server = GitHubServer::new()
        .with_default_github_app()
        .with_default_app_installations();

    let mut pull = server.make_pull("test/repo", &["wiki/Article/en.md"]);
    let pull_number = pull.number;
    server = server.with_pull("test/repo", &pull);
    pull.diff = None;

    let c = new_controller(&server, true).await;
    c.upsert_pull("test/repo", pull, false).await.unwrap();

    let m = c.memory.pulls("test/repo");
    assert!(m.is_some());
    assert!(m.unwrap().get(&pull_number).unwrap().diff.is_some());
}

#[tokio::test]
async fn test_simple_overlap_originals() {
    let mut server = GitHubServer::new()
        .with_default_github_app()
        .with_default_app_installations();

    let pulls = vec![
        server.make_pull("test/repo", &["wiki/Article/en.md"]),
        server.make_pull("test/repo", &["wiki/Article/en.md"]),
    ];
    server = server.with_pulls("test/repo", &pulls);

    let c = new_controller(&server, true).await;
    for p in pulls {
        c.upsert_pull("test/repo", p, false).await.unwrap();
    }

    assert!(c.conflicts.by_trigger("test/repo", 1).is_empty());
    assert_eq!(
        &c.conflicts.by_trigger("test/repo", 2),
        &vec![Conflict::overlap(
            2,
            1,
            server.url.pull_url("test/repo", 1),
            vec!["wiki/Article/en.md".to_string()]
        )]
    );
}

#[tokio::test]
async fn test_simple_overlap_translations() {
    let mut server = GitHubServer::new()
        .with_default_github_app()
        .with_default_app_installations();

    let pulls = vec![
        server.make_pull("test/repo", &["wiki/Article/ru.md"]),
        server.make_pull("test/repo", &["wiki/Article/ru.md"]),
    ];
    server = server.with_pulls("test/repo", &pulls);

    let c = new_controller(&server, true).await;
    for p in pulls {
        c.upsert_pull("test/repo", p, false).await.unwrap();
    }

    assert!(c.conflicts.by_trigger("test/repo", 1).is_empty());
    assert_eq!(
        &c.conflicts.by_trigger("test/repo", 2),
        &vec![Conflict::overlap(
            2,
            1,
            server.url.pull_url("test/repo", 1),
            vec!["wiki/Article/ru.md".to_string()]
        )]
    );
}

#[tokio::test]
async fn test_different_translations_do_not_overlap() {
    let mut server = GitHubServer::new()
        .with_default_github_app()
        .with_default_app_installations();

    let pulls = vec![
        server.make_pull("test/repo", &["wiki/Article/ru.md"]),
        server.make_pull("test/repo", &["wiki/Article/ko.md"]),
    ];
    server = server.with_pulls("test/repo", &pulls);

    let c = new_controller(&server, true).await;
    for p in pulls {
        c.upsert_pull("test/repo", p, false).await.unwrap();
    }

    assert!(c.conflicts.by_trigger("test/repo", 1).is_empty());
    assert!(c.conflicts.by_trigger("test/repo", 2).is_empty());
}

#[tokio::test]
async fn test_simple_early_incomplete_translation() {
    let mut server = GitHubServer::new()
        .with_default_github_app()
        .with_default_app_installations();

    let pulls = vec![
        server.make_pull("test/repo", &["wiki/Article/ru.md"]),
        server.make_pull("test/repo", &["wiki/Article/en.md"]),
    ];
    server = server.with_pulls("test/repo", &pulls);

    let c = new_controller(&server, true).await;
    for p in pulls {
        c.upsert_pull("test/repo", p, false).await.unwrap();
    }

    assert_eq!(
        &c.conflicts.by_trigger("test/repo", 1),
        &vec![Conflict::incomplete_translation(
            1,
            2,
            server.url.pull_url("test/repo", 2),
            vec!["wiki/Article/en.md".to_string()]
        )]
    );
    assert!(c.conflicts.by_trigger("test/repo", 2).is_empty());
}

#[tokio::test]
async fn test_simple_late_incomplete_translation() {
    let mut server = GitHubServer::new()
        .with_default_github_app()
        .with_default_app_installations();

    let pulls = vec![
        server.make_pull("test/repo", &["wiki/Article/en.md"]),
        server.make_pull("test/repo", &["wiki/Article/ru.md"]),
    ];
    server = server.with_pulls("test/repo", &pulls);

    let c = new_controller(&server, true).await;
    for p in pulls {
        c.upsert_pull("test/repo", p, false).await.unwrap();
    }

    assert!(&c.conflicts.by_trigger("test/repo", 1).is_empty());
    assert_eq!(
        &c.conflicts.by_trigger("test/repo", 2),
        &vec![Conflict::incomplete_translation(
            2,
            1,
            server.url.pull_url("test/repo", 1),
            vec!["wiki/Article/en.md".to_string()]
        )]
    );
}

#[tokio::test]
async fn test_multiple_overlapping_changes() {
    let mut server = GitHubServer::new()
        .with_default_github_app()
        .with_default_app_installations();

    let pulls = vec![
        server.make_pull("test/repo", &["wiki/Article/en.md"]),
        server.make_pull("test/repo", &["wiki/Article/en.md"]),
        server.make_pull("test/repo", &["wiki/Article/en.md"]),
        server.make_pull("test/repo", &["wiki/Article/en.md"]),
    ];
    server = server.with_pulls("test/repo", &pulls);

    let c = new_controller(&server, true).await;
    for p in pulls {
        c.upsert_pull("test/repo", p, false).await.unwrap();
    }

    let overlap = |trigger, original| {
        Conflict::overlap(
            trigger,
            original,
            server.url.pull_url("test/repo", original),
            vec!["wiki/Article/en.md".to_string()],
        )
    };

    assert!(&c.conflicts.by_trigger("test/repo", 1).is_empty());
    assert_eq!(
        &c.conflicts.by_trigger("test/repo", 2),
        &vec![overlap(2, 1)]
    );
    assert_eq!(
        &c.conflicts.by_trigger("test/repo", 3),
        &vec![overlap(3, 1), overlap(3, 2),]
    );
    assert_eq!(
        &c.conflicts.by_trigger("test/repo", 4),
        &vec![overlap(4, 1), overlap(4, 2), overlap(4, 3),]
    );
}

#[tokio::test]
async fn test_multiple_incomplete_translations() {
    let mut server = GitHubServer::new()
        .with_default_github_app()
        .with_default_app_installations();

    let pulls = vec![
        server.make_pull("test/repo", &["wiki/Article/ru.md"]),
        server.make_pull("test/repo", &["wiki/Article/jp.md"]),
        server.make_pull("test/repo", &["wiki/Article/ko.md"]),
        server.make_pull("test/repo", &["wiki/Article/en.md"]),
    ];
    server = server.with_pulls("test/repo", &pulls);

    let c = new_controller(&server, true).await;
    for p in pulls {
        c.upsert_pull("test/repo", p, false).await.unwrap();
    }

    let incomplete_translation = |trigger, original| {
        Conflict::incomplete_translation(
            trigger,
            original,
            server.url.pull_url("test/repo", original),
            vec!["wiki/Article/en.md".to_string()],
        )
    };

    assert_eq!(
        &c.conflicts.by_trigger("test/repo", 1),
        &vec![incomplete_translation(1, 4)]
    );
    assert_eq!(
        &c.conflicts.by_trigger("test/repo", 2),
        &vec![incomplete_translation(2, 4)]
    );
    assert_eq!(
        &c.conflicts.by_trigger("test/repo", 3),
        &vec![incomplete_translation(3, 4)]
    );
    assert!(&c.conflicts.by_trigger("test/repo", 4).is_empty());
}

#[tokio::test]
async fn test_incomplete_translation_multiple_conflicts() {
    let mut server = GitHubServer::new()
        .with_default_github_app()
        .with_default_app_installations();

    let pulls = vec![
        server.make_pull("test/repo", &["wiki/Article/en.md"]),
        server.make_pull("test/repo", &["wiki/Article/ru.md"]),
        server.make_pull("test/repo", &["wiki/Article/jp.md"]),
        server.make_pull("test/repo", &["wiki/Article/ko.md"]),
    ];
    server = server.with_pulls("test/repo", &pulls);

    let c = new_controller(&server, true).await;
    for p in pulls {
        c.upsert_pull("test/repo", p, false).await.unwrap();
    }

    let incomplete_translation_conflict = |trigger, original| {
        Conflict::incomplete_translation(
            trigger,
            original,
            server.url.pull_url("test/repo", original),
            vec![format!("wiki/Article/en.md")],
        )
    };

    assert!(&c.conflicts.by_trigger("test/repo", 1).is_empty());
    assert_eq!(
        &c.conflicts.by_trigger("test/repo", 2),
        &vec![incomplete_translation_conflict(2, 1)]
    );
    assert_eq!(
        &c.conflicts.by_trigger("test/repo", 3),
        &vec![incomplete_translation_conflict(3, 1)]
    );
    assert_eq!(
        &c.conflicts.by_trigger("test/repo", 4),
        &vec![incomplete_translation_conflict(4, 1)]
    );
}

#[tokio::test]
async fn test_overlap_no_extra_files_on_update() {
    let mut server = GitHubServer::new()
        .with_default_github_app()
        .with_default_app_installations();

    let pulls = vec![
        server.make_pull("test/repo", &["wiki/Article/ru.md"]),
        server.make_pull("test/repo", &["wiki/Article/ru.md"]),
    ];
    server = server.with_pulls("test/repo", &pulls);

    let c = new_controller(&server, true).await;
    for p in pulls.iter() {
        c.upsert_pull("test/repo", p.clone(), false).await.unwrap();
    }

    server.change_pull_diff(
        "test/repo",
        1,
        &["wiki/Article/ru.md", "wiki/Other_article/ru.md"],
    );
    c.upsert_pull("test/repo", pulls[0].clone(), false)
        .await
        .unwrap();

    assert!(&c.conflicts.by_trigger("test/repo", 1).is_empty());
    assert_eq!(
        &c.conflicts.by_trigger("test/repo", 2),
        &vec![Conflict::overlap(
            2,
            1,
            server.url.pull_url("test/repo", 1),
            vec!["wiki/Article/ru.md".to_string()]
        )]
    );
}

#[tokio::test]
async fn test_overlap_file_set_update_in_trigger_recognized() {
    let mut server = GitHubServer::new()
        .with_default_github_app()
        .with_default_app_installations();

    let pulls = vec![
        server.make_pull("test/repo", &["wiki/Article/ru.md"]),
        server.make_pull(
            "test/repo",
            &["wiki/Article/ru.md", "wiki/Other_article/en.md"],
        ),
    ];
    server = server.with_pulls("test/repo", &pulls);

    let c = new_controller(&server, true).await;
    for p in pulls.iter() {
        c.upsert_pull("test/repo", p.clone(), false).await.unwrap();
    }

    let u1 = server.change_pull_diff(
        "test/repo",
        1,
        &["wiki/Article/ru.md", "wiki/Other_article/en.md"],
    );
    server = server.with_pull("test/repo", &u1);
    c.upsert_pull("test/repo", u1, false).await.unwrap();

    assert!(&c.conflicts.by_trigger("test/repo", 1).is_empty());
    assert_eq!(
        &c.conflicts.by_trigger("test/repo", 2),
        &vec![Conflict::overlap(
            2,
            1,
            server.url.pull_url("test/repo", 1),
            vec![
                "wiki/Article/ru.md".to_string(),
                "wiki/Other_article/en.md".to_string()
            ]
        )]
    );
}

#[tokio::test]
async fn test_overlap_double_update_recognized() {
    let mut server = GitHubServer::new()
        .with_default_github_app()
        .with_default_app_installations();

    let pulls = vec![
        server.make_pull("test/repo", &["wiki/Article/ru.md"]),
        server.make_pull("test/repo", &["wiki/Article/ru.md"]),
    ];
    server = server.with_pulls("test/repo", &pulls);

    let c = new_controller(&server, true).await;
    for p in pulls.iter() {
        c.upsert_pull("test/repo", p.clone(), false).await.unwrap();
    }

    let u1 = server.change_pull_diff(
        "test/repo",
        1,
        &["wiki/Article/ru.md", "wiki/Other_article/en.md"],
    );
    server = server.with_pull("test/repo", &u1);
    c.upsert_pull("test/repo", u1.clone(), false).await.unwrap();

    let u2 = server.change_pull_diff(
        "test/repo",
        2,
        &["wiki/Article/ru.md", "wiki/Other_article/en.md"],
    );
    server = server.with_pull("test/repo", &u2);
    c.upsert_pull("test/repo", u2.clone(), false).await.unwrap();

    assert!(&c.conflicts.by_trigger("test/repo", 1).is_empty());
    assert_eq!(
        &c.conflicts.by_trigger("test/repo", 2),
        &vec![Conflict::overlap(
            2,
            1,
            server.url.pull_url("test/repo", 1),
            vec![
                "wiki/Article/ru.md".to_string(),
                "wiki/Other_article/en.md".to_string()
            ]
        )]
    );
}

#[tokio::test]
async fn test_early_incomplete_translation_update_no_unrelated_files() {
    let mut server = GitHubServer::new()
        .with_default_github_app()
        .with_default_app_installations();

    let pulls = vec![
        server.make_pull("test/repo", &["wiki/Article/ru.md"]),
        server.make_pull("test/repo", &["wiki/Article/en.md"]),
    ];
    server = server.with_pulls("test/repo", &pulls);

    let c = new_controller(&server, true).await;
    for p in pulls.iter() {
        c.upsert_pull("test/repo", p.clone(), false).await.unwrap();
    }

    let u2 = server.change_pull_diff(
        "test/repo",
        2,
        &["wiki/Article/en.md", "wiki/Other_article/en.md"],
    );
    server = server.with_pull("test/repo", &u2);
    c.upsert_pull("test/repo", u2, false).await.unwrap();

    assert!(&c.conflicts.by_trigger("test/repo", 2).is_empty());
    assert_eq!(
        &c.conflicts.by_trigger("test/repo", 1),
        &vec![Conflict::incomplete_translation(
            1,
            2,
            server.url.pull_url("test/repo", 2),
            vec!["wiki/Article/en.md".to_string()]
        )]
    );
}

#[tokio::test]
async fn test_incomplete_translation_original_update_recognized() {
    let mut server = GitHubServer::new()
        .with_default_github_app()
        .with_default_app_installations();

    let pulls = vec![
        server.make_pull(
            "test/repo",
            &["wiki/Article/ru.md", "wiki/Other_article/sv.md"],
        ),
        server.make_pull("test/repo", &["wiki/Article/en.md"]),
    ];
    server = server.with_pulls("test/repo", &pulls);

    let c = new_controller(&server, true).await;
    for p in pulls.iter() {
        c.upsert_pull("test/repo", p.clone(), false).await.unwrap();
    }

    let u2 = server.change_pull_diff(
        "test/repo",
        2,
        &["wiki/Article/en.md", "wiki/Other_article/en.md"],
    );
    server = server.with_pull("test/repo", &u2);
    c.upsert_pull("test/repo", u2, false).await.unwrap();

    assert!(&c.conflicts.by_trigger("test/repo", 2).is_empty());
    assert_eq!(
        &c.conflicts.by_trigger("test/repo", 1),
        &vec![Conflict::incomplete_translation(
            1,
            2,
            server.url.pull_url("test/repo", 2),
            vec![
                "wiki/Article/en.md".to_string(),
                "wiki/Other_article/en.md".to_string()
            ]
        )]
    );
}

#[tokio::test]
async fn test_incomplete_translation_double_update() {
    let mut server = GitHubServer::new()
        .with_default_github_app()
        .with_default_app_installations();

    let pulls = vec![
        server.make_pull("test/repo", &["wiki/Article/ru.md"]),
        server.make_pull("test/repo", &["wiki/Article/en.md"]),
    ];
    server = server.with_pulls("test/repo", &pulls);

    let c = new_controller(&server, true).await;
    for p in pulls.iter() {
        c.upsert_pull("test/repo", p.clone(), false).await.unwrap();
    }

    let u1 = server.change_pull_diff(
        "test/repo",
        1,
        &["wiki/Article/ru.md", "wiki/Other_article/ru.md"],
    );
    server = server.with_pull("test/repo", &u1);
    c.upsert_pull("test/repo", u1, false).await.unwrap();

    let u2 = server.change_pull_diff(
        "test/repo",
        2,
        &["wiki/Article/en.md", "wiki/Other_article/en.md"],
    );
    server = server.with_pull("test/repo", &u2);
    c.upsert_pull("test/repo", u2, false).await.unwrap();

    assert!(&c.conflicts.by_trigger("test/repo", 2).is_empty());
    assert_eq!(
        &c.conflicts.by_trigger("test/repo", 1),
        &vec![Conflict::incomplete_translation(
            1,
            2,
            server.url.pull_url("test/repo", 2),
            vec![
                "wiki/Article/en.md".to_string(),
                "wiki/Other_article/en.md".to_string()
            ]
        )]
    );
}

#[tokio::test]
async fn test_late_incomplete_translation_update_no_extra_files() {
    let mut server = GitHubServer::new()
        .with_default_github_app()
        .with_default_app_installations();

    let pulls = vec![
        server.make_pull("test/repo", &["wiki/Article/en.md"]),
        server.make_pull("test/repo", &["wiki/Article/ru.md"]),
    ];
    server = server.with_pulls("test/repo", &pulls);

    let c = new_controller(&server, true).await;
    for p in pulls.iter() {
        c.upsert_pull("test/repo", p.clone(), false).await.unwrap();
    }

    let u1 = server.change_pull_diff(
        "test/repo",
        1,
        &["wiki/Article/en.md", "wiki/Other_article/en.md"],
    );
    server = server.with_pull("test/repo", &u1);
    c.upsert_pull("test/repo", u1, false).await.unwrap();

    assert!(&c.conflicts.by_trigger("test/repo", 1).is_empty());
    assert_eq!(
        &c.conflicts.by_trigger("test/repo", 2),
        &vec![Conflict::incomplete_translation(
            2,
            1,
            server.url.pull_url("test/repo", 1),
            vec!["wiki/Article/en.md".to_string()]
        )]
    );
}

#[tokio::test]
async fn test_incomplete_translation_update_recognized() {
    let mut server = GitHubServer::new()
        .with_default_github_app()
        .with_default_app_installations();

    let pulls = vec![
        server.make_pull(
            "test/repo",
            &["wiki/Article/en.md", "wiki/Other_article/en.md"],
        ),
        server.make_pull("test/repo", &["wiki/Article/ru.md"]),
    ];
    server = server.with_pulls("test/repo", &pulls);

    let c = new_controller(&server, true).await;
    for p in pulls.iter() {
        c.upsert_pull("test/repo", p.clone(), false).await.unwrap();
    }

    let u2 = server.change_pull_diff(
        "test/repo",
        2,
        &["wiki/Article/ru.md", "wiki/Other_article/ru.md"],
    );
    server = server.with_pull("test/repo", &u2);
    c.upsert_pull("test/repo", u2, false).await.unwrap();

    assert!(&c.conflicts.by_trigger("test/repo", 1).is_empty());
    assert_eq!(
        &c.conflicts.by_trigger("test/repo", 2),
        &vec![Conflict::incomplete_translation(
            2,
            1,
            server.url.pull_url("test/repo", 1),
            vec![
                "wiki/Article/en.md".to_string(),
                "wiki/Other_article/en.md".to_string()
            ]
        )]
    );
}

#[tokio::test]
async fn test_outdated_translation_produces_single_conflict() {
    let mut server = GitHubServer::new()
        .with_default_github_app()
        .with_default_app_installations();

    let pulls = vec![
        server.make_pull("test/repo", &["wiki/Article/en.md", "wiki/Article/ru.md"]),
        server.make_pull("test/repo", &["wiki/Article/ru.md"]),
    ];
    server = server.with_pulls("test/repo", &pulls);

    let c = new_controller(&server, true).await;
    for p in pulls.iter() {
        c.upsert_pull("test/repo", p.clone(), false).await.unwrap();
    }

    assert!(&c.conflicts.by_trigger("test/repo", 1).is_empty());
    assert_eq!(
        &c.conflicts.by_trigger("test/repo", 2),
        &vec![Conflict::incomplete_translation(
            2,
            1,
            server.url.pull_url("test/repo", 1),
            vec!["wiki/Article/en.md".to_string(),]
        )]
    );
}

#[tokio::test]
async fn test_three_conflicts_at_once() {
    let mut server = GitHubServer::new()
        .with_default_github_app()
        .with_default_app_installations();

    let pulls = vec![
        server.make_pull("test/repo", &["wiki/Article/en.md"]),
        server.make_pull("test/repo", &["wiki/Other_article/ru.md"]),
        server.make_pull(
            "test/repo",
            &[
                "wiki/Article/ru.md",
                "wiki/Other_article/ru.md",
                "wiki/Different_article/ru.md",
            ],
        ),
        server.make_pull("test/repo", &["wiki/Different_article/en.md"]),
    ];
    server = server.with_pulls("test/repo", &pulls);

    let c = new_controller(&server, true).await;
    for p in pulls.iter() {
        c.upsert_pull("test/repo", p.clone(), false).await.unwrap();
    }

    assert_eq!(
        &c.conflicts.by_trigger("test/repo", 3),
        &vec![
            Conflict::overlap(
                3,
                2,
                server.url.pull_url("test/repo", 2),
                vec!["wiki/Other_article/ru.md".to_string(),]
            ),
            Conflict::incomplete_translation(
                3,
                1,
                server.url.pull_url("test/repo", 1),
                vec!["wiki/Article/en.md".to_string(),]
            ),
            Conflict::incomplete_translation(
                3,
                4,
                server.url.pull_url("test/repo", 4),
                vec!["wiki/Different_article/en.md".to_string(),]
            ),
        ]
    );
}

#[tokio::test]
async fn test_closed_pull_is_removed() {
    let mut server = GitHubServer::new()
        .with_default_github_app()
        .with_default_app_installations();

    let pull = server.make_pull("test/repo", &["wiki/Article/en.md", "wiki/Article_2/ru.md"]);
    server = server.with_pull("test/repo", &pull);

    let c = new_controller(&server, true).await;
    c.upsert_pull("test/repo", pull.clone(), true)
        .await
        .unwrap();

    c.remove_pull("test/repo", pull);
    assert!(c.memory.pulls("test/repo").unwrap().is_empty());
}

#[tokio::test]
async fn test_closed_pull_conflicts_removed() {
    let mut server = GitHubServer::new()
        .with_default_github_app()
        .with_default_app_installations();

    let pulls = vec![
        server.make_pull("test/repo", &["wiki/Article/en.md", "wiki/Article_2/ru.md"]),
        server.make_pull("test/repo", &["wiki/Other_article/en.md"]),
        server.make_pull(
            "test/repo",
            &["wiki/Article/ru.md", "wiki/Other_article/ru.md"],
        ),
    ];
    server = server.with_pulls("test/repo", &pulls);

    let c = new_controller(&server, true).await;
    for p in pulls.iter() {
        c.upsert_pull("test/repo", p.clone(), false).await.unwrap();
    }

    c.remove_pull("test/repo", pulls[2].clone());
    assert!(&c.conflicts.by_trigger("test/repo", 3).is_empty());
}

#[tokio::test]
async fn test_closed_pull_related_conflicts_removed() {
    let mut server = GitHubServer::new()
        .with_default_github_app()
        .with_default_app_installations();

    let pulls = vec![
        server.make_pull("test/repo", &["wiki/Article/en.md"]),
        server.make_pull("test/repo", &["wiki/Article/ru.md"]),
        server.make_pull("test/repo", &["wiki/Article/cz.md"]),
        server.make_pull("test/repo", &["wiki/Article/zh.md"]),
    ];
    server = server.with_pulls("test/repo", &pulls);

    let c = new_controller(&server, true).await;
    for p in pulls.iter() {
        c.upsert_pull("test/repo", p.clone(), false).await.unwrap();
    }

    c.remove_pull("test/repo", pulls[0].clone());
    for p in pulls.iter().skip(1) {
        assert!(c.conflicts.by_original("test/repo", p.number).is_empty());
        assert!(c.conflicts.by_trigger("test/repo", p.number).is_empty());
    }
}

#[allow(unused_assignments)]
#[tokio::test]
async fn test_obsolete_conflict_removed() {
    let mut server = GitHubServer::new()
        .with_default_github_app()
        .with_default_app_installations();

    let pulls = vec![
        server.make_pull("test/repo", &["wiki/Article/ru.md"]),
        server.make_pull("test/repo", &["wiki/Article/ru.md"]),
    ];
    server = server.with_pulls("test/repo", &pulls);

    let c = new_controller(&server, true).await;
    for p in pulls.iter() {
        c.upsert_pull("test/repo", p.clone(), false).await.unwrap();
    }

    let u1 = server.change_pull_diff("test/repo", 1, &["wiki/Other_article/ru.md"]);
    server = server.with_pull("test/repo", &u1);
    c.upsert_pull("test/repo", u1, false).await.unwrap();

    assert!(c.conflicts.by_original("test/repo", 1).is_empty());
    assert!(c.conflicts.by_original("test/repo", 2).is_empty());
}

#[tokio::test]
async fn test_only_obsolete_conflict_is_removed_overlap() {
    let mut server = GitHubServer::new()
        .with_default_github_app()
        .with_default_app_installations();

    let pulls = [
        server.make_pull(
            "test/repo",
            &["wiki/Article/ru.md", "wiki/Article/Other_article/en.md"],
        ),
        server.make_pull(
            "test/repo",
            &["wiki/Article/ru.md", "wiki/Article/Other_article/ru.md"],
        ),
    ];
    server = server.with_pulls("test/repo", &pulls);

    let c = new_controller(&server, true).await;
    for p in pulls.iter() {
        c.upsert_pull("test/repo", p.clone(), false).await.unwrap();
    }

    let u1 = server.change_pull_diff("test/repo", 1, &["wiki/Article/Other_article/en.md"]);
    server = server.with_pull("test/repo", &u1);
    c.upsert_pull("test/repo", u1, false).await.unwrap();

    assert_eq!(
        &c.conflicts.by_trigger("test/repo", 2),
        &vec![Conflict::incomplete_translation(
            2,
            1,
            server.url.pull_url("test/repo", 1),
            vec!["wiki/Article/Other_article/en.md".to_string()]
        )]
    );
}

#[tokio::test]
async fn test_only_obsolete_conflict_is_removed_incomplete_translation() {
    let mut server = GitHubServer::new()
        .with_default_github_app()
        .with_default_app_installations();

    let pulls = [
        server.make_pull(
            "test/repo",
            &["wiki/Article/ru.md", "wiki/Article/Other_article/en.md"],
        ),
        server.make_pull(
            "test/repo",
            &["wiki/Article/ru.md", "wiki/Article/Other_article/ru.md"],
        ),
    ];
    server = server.with_pulls("test/repo", &pulls);

    let c = new_controller(&server, true).await;
    for p in pulls.iter() {
        c.upsert_pull("test/repo", p.clone(), false).await.unwrap();
    }

    let u1 = server.change_pull_diff("test/repo", 1, &["wiki/Article/ru.md"]);
    server = server.with_pull("test/repo", &u1);
    c.upsert_pull("test/repo", u1, false).await.unwrap();

    assert!(c.conflicts.by_trigger("test/repo", 1).is_empty());
    assert_eq!(
        &c.conflicts.by_trigger("test/repo", 2),
        &vec![Conflict::overlap(
            2,
            1,
            server.url.pull_url("test/repo", 1),
            vec!["wiki/Article/ru.md".to_string()]
        )]
    );
}
