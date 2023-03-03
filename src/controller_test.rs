use super::*;

use crate::helpers::pulls::Conflict;
use crate::test::{self, pull_link};

async fn make_controller(init: bool) -> Controller<test::DummyGitHubClient> {
    let mut c = Controller::<test::DummyGitHubClient>::new(
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

#[tokio::test]
async fn test_has_control_over() {
    let c = make_controller(true).await;

    assert!(c.has_control_over(&structs::Actor {
        id: 1,
        login: "test-app[bot]".to_string()
    }));
    assert!(!c.has_control_over(&structs::Actor {
        id: 1,
        login: "test-app".to_string()
    }));
    assert!(!c.has_control_over(&structs::Actor {
        id: 2,
        login: "ppy".to_string()
    }));
}

#[tokio::test]
async fn test_add_pull() {
    let c = make_controller(true).await;
    let mut p = c.github.test_add_pull("test/repo", &["wiki/Article/en.md"]);
    p.diff = None;
    let pn = p.number;

    c.add_pull("test/repo", p, false).await.unwrap();

    let m = c.memory.pulls("test/repo");
    assert!(m.is_some());
    assert!(m.unwrap().get(&pn).unwrap().diff.is_some());
}

#[tokio::test]
async fn test_simple_existing_change() {
    let c = make_controller(true).await;
    let pulls = [
        c.github.test_add_pull("test/repo", &["wiki/Article/en.md"]),
        c.github.test_add_pull("test/repo", &["wiki/Article/en.md"]),
    ];
    for p in pulls {
        c.add_pull("test/repo", p, false).await.unwrap();
    }

    assert!(c.memory.conflicts("test/repo").get(&1).is_none());
    assert_eq!(
        c.memory.conflicts("test/repo").get(&2).unwrap(),
        &vec![Conflict::existing_change(
            2,
            1,
            pull_link("test/repo", 1),
            vec!["wiki/Article/en.md".to_string()]
        )]
    );
}

#[tokio::test]
async fn test_simple_new_original_change() {
    let c = make_controller(true).await;
    let pulls = [
        c.github.test_add_pull("test/repo", &["wiki/Article/ru.md"]),
        c.github.test_add_pull("test/repo", &["wiki/Article/en.md"]),
    ];
    for p in pulls {
        c.add_pull("test/repo", p, false).await.unwrap();
    }

    assert_eq!(
        c.memory.conflicts("test/repo").get(&1).unwrap(),
        &vec![Conflict::new_original_change(
            1,
            2,
            pull_link("test/repo", 2),
            vec!["wiki/Article/en.md".to_string()]
        )]
    );
    assert!(c.memory.conflicts("test/repo").get(&2).is_none());
}

#[tokio::test]
async fn test_simple_existing_original_change() {
    let c = make_controller(true).await;
    let pulls = [
        c.github.test_add_pull("test/repo", &["wiki/Article/en.md"]),
        c.github.test_add_pull("test/repo", &["wiki/Article/ru.md"]),
    ];
    for p in pulls {
        c.add_pull("test/repo", p, false).await.unwrap();
    }

    assert!(c.memory.conflicts("test/repo").get(&1).is_none());
    assert_eq!(
        c.memory.conflicts("test/repo").get(&2).unwrap(),
        &vec![Conflict::existing_original_change(
            2,
            1,
            pull_link("test/repo", 1),
            vec!["wiki/Article/ru.md".to_string()],
        )]
    );
}

#[tokio::test]
async fn test_existing_change_multiple_conflicts() {
    let c = make_controller(true).await;
    let pulls = [
        c.github.test_add_pull("test/repo", &["wiki/Article/en.md"]),
        c.github.test_add_pull("test/repo", &["wiki/Article/en.md"]),
        c.github.test_add_pull("test/repo", &["wiki/Article/en.md"]),
        c.github.test_add_pull("test/repo", &["wiki/Article/en.md"]),
    ];
    for p in pulls {
        c.add_pull("test/repo", p, false).await.unwrap();
    }

    let existing_change_conflict = |trigger, original| {
        Conflict::existing_change(
            trigger,
            original,
            pull_link("test/repo", original),
            vec!["wiki/Article/en.md".to_string()],
        )
    };

    assert!(c.memory.conflicts("test/repo").get(&1).is_none());
    assert_eq!(
        c.memory.conflicts("test/repo").get(&2).unwrap(),
        &vec![existing_change_conflict(2, 1)]
    );
    assert_eq!(
        c.memory.conflicts("test/repo").get(&3).unwrap(),
        &vec![
            existing_change_conflict(3, 1),
            existing_change_conflict(3, 2),
        ]
    );
    assert_eq!(
        c.memory.conflicts("test/repo").get(&4).unwrap(),
        &vec![
            existing_change_conflict(4, 1),
            existing_change_conflict(4, 2),
            existing_change_conflict(4, 3),
        ]
    );
}

#[tokio::test]
async fn test_new_original_change_multiple_conflicts() {
    let c = make_controller(true).await;
    let pulls = [
        c.github.test_add_pull("test/repo", &["wiki/Article/ru.md"]),
        c.github.test_add_pull("test/repo", &["wiki/Article/jp.md"]),
        c.github.test_add_pull("test/repo", &["wiki/Article/ko.md"]),
        c.github.test_add_pull("test/repo", &["wiki/Article/en.md"]),
    ];
    for p in pulls {
        c.add_pull("test/repo", p, false).await.unwrap();
    }

    let new_original_change_conflict = |trigger, original| {
        Conflict::new_original_change(
            trigger,
            original,
            pull_link("test/repo", original),
            vec!["wiki/Article/en.md".to_string()],
        )
    };

    assert_eq!(
        c.memory.conflicts("test/repo").get(&1).unwrap(),
        &vec![new_original_change_conflict(1, 4)]
    );
    assert_eq!(
        c.memory.conflicts("test/repo").get(&2).unwrap(),
        &vec![new_original_change_conflict(2, 4)]
    );
    assert_eq!(
        c.memory.conflicts("test/repo").get(&3).unwrap(),
        &vec![new_original_change_conflict(3, 4)]
    );
    assert!(c.memory.conflicts("test/repo").get(&4).is_none());
}

#[tokio::test]
async fn test_existing_original_change_multiple_conflicts() {
    let c = make_controller(true).await;
    let pulls = [
        c.github.test_add_pull("test/repo", &["wiki/Article/en.md"]),
        c.github.test_add_pull("test/repo", &["wiki/Article/ru.md"]),
        c.github.test_add_pull("test/repo", &["wiki/Article/jp.md"]),
        c.github.test_add_pull("test/repo", &["wiki/Article/ko.md"]),
    ];
    for p in pulls {
        c.add_pull("test/repo", p, false).await.unwrap();
    }

    let existing_original_change_conflict = |trigger, original, language| {
        Conflict::existing_original_change(
            trigger,
            original,
            pull_link("test/repo", original),
            vec![format!("wiki/Article/{}.md", language)],
        )
    };

    assert!(c.memory.conflicts("test/repo").get(&1).is_none());
    assert_eq!(
        c.memory.conflicts("test/repo").get(&2).unwrap(),
        &vec![existing_original_change_conflict(2, 1, "ru")]
    );
    assert_eq!(
        c.memory.conflicts("test/repo").get(&3).unwrap(),
        &vec![existing_original_change_conflict(3, 1, "jp")]
    );
    assert_eq!(
        c.memory.conflicts("test/repo").get(&4).unwrap(),
        &vec![existing_original_change_conflict(4, 1, "ko")]
    );
}

#[tokio::test]
async fn test_existing_change_conflict_update_no_extra_files() {
    let c = make_controller(true).await;
    let pulls = [
        c.github.test_add_pull("test/repo", &["wiki/Article/ru.md"]),
        c.github.test_add_pull("test/repo", &["wiki/Article/ru.md"]),
    ];
    for p in pulls.iter() {
        c.add_pull("test/repo", p.clone(), false).await.unwrap();
    }

    c.github.test_update_pull(
        "test/repo",
        1,
        &["wiki/Article/ru.md", "wiki/Other_article/ru.md"],
    );
    c.add_pull("test/repo", pulls[0].clone(), false)
        .await
        .unwrap();

    assert!(c.memory.conflicts("test/repo").get(&1).is_none());
    assert_eq!(
        c.memory.conflicts("test/repo").get(&2).unwrap(),
        &vec![Conflict::existing_change(
            2,
            1,
            pull_link("test/repo", 1),
            vec!["wiki/Article/ru.md".to_string()]
        )]
    );
}

#[tokio::test]
async fn test_existing_change_conflict_update_recognized() {
    let c = make_controller(true).await;
    let pulls = [
        c.github.test_add_pull("test/repo", &["wiki/Article/ru.md"]),
        c.github.test_add_pull(
            "test/repo",
            &["wiki/Article/ru.md", "wiki/Other_article/en.md"],
        ),
    ];
    for p in pulls.iter() {
        c.add_pull("test/repo", p.clone(), false).await.unwrap();
    }

    c.github.test_update_pull(
        "test/repo",
        1,
        &["wiki/Article/ru.md", "wiki/Other_article/en.md"],
    );
    c.add_pull("test/repo", pulls[0].clone(), false)
        .await
        .unwrap();

    assert!(c.memory.conflicts("test/repo").get(&1).is_none());
    assert_eq!(
        c.memory.conflicts("test/repo").get(&2).unwrap(),
        &vec![Conflict::existing_change(
            2,
            1,
            pull_link("test/repo", 1),
            vec![
                "wiki/Article/ru.md".to_string(),
                "wiki/Other_article/en.md".to_string()
            ]
        )]
    );
}

#[tokio::test]
async fn test_existing_change_conflict_double_update() {
    let c = make_controller(true).await;
    let pulls = [
        c.github.test_add_pull("test/repo", &["wiki/Article/ru.md"]),
        c.github.test_add_pull("test/repo", &["wiki/Article/ru.md"]),
    ];
    for p in pulls.iter() {
        c.add_pull(
            "test/repo",
            c.github.fetch_pull("test/repo", p.number),
            false,
        )
        .await
        .unwrap();
    }

    c.github.test_update_pull(
        "test/repo",
        1,
        &["wiki/Article/ru.md", "wiki/Other_article/en.md"],
    );
    c.add_pull("test/repo", c.github.fetch_pull("test/repo", 1), false)
        .await
        .unwrap();
    c.github.test_update_pull(
        "test/repo",
        2,
        &["wiki/Article/ru.md", "wiki/Other_article/en.md"],
    );
    c.add_pull("test/repo", c.github.fetch_pull("test/repo", 2), false)
        .await
        .unwrap();

    assert!(c.memory.conflicts("test/repo").get(&1).is_none());
    assert_eq!(
        c.memory.conflicts("test/repo").get(&2).unwrap(),
        &vec![Conflict::existing_change(
            2,
            1,
            pull_link("test/repo", 1),
            vec![
                "wiki/Article/ru.md".to_string(),
                "wiki/Other_article/en.md".to_string()
            ]
        )]
    );
}

#[tokio::test]
async fn test_new_original_change_conflict_update_no_extra_files() {
    let c = make_controller(true).await;
    let pulls = [
        c.github.test_add_pull("test/repo", &["wiki/Article/ru.md"]),
        c.github.test_add_pull("test/repo", &["wiki/Article/en.md"]),
    ];
    for p in pulls.iter() {
        c.add_pull(
            "test/repo",
            c.github.fetch_pull("test/repo", p.number),
            false,
        )
        .await
        .unwrap();
    }

    c.github.test_update_pull(
        "test/repo",
        2,
        &["wiki/Article/en.md", "wiki/Other_article/en.md"],
    );
    c.add_pull("test/repo", c.github.fetch_pull("test/repo", 2), false)
        .await
        .unwrap();

    assert!(c.memory.conflicts("test/repo").get(&2).is_none());
    assert_eq!(
        c.memory.conflicts("test/repo").get(&1).unwrap(),
        &vec![Conflict::new_original_change(
            1,
            2,
            pull_link("test/repo", 2),
            vec!["wiki/Article/en.md".to_string()]
        )]
    );
}

#[tokio::test]
async fn test_new_original_change_conflict_update_recognized() {
    let c = make_controller(true).await;
    let pulls = [
        c.github.test_add_pull(
            "test/repo",
            &["wiki/Article/ru.md", "wiki/Other_article/sv.md"],
        ),
        c.github.test_add_pull("test/repo", &["wiki/Article/en.md"]),
    ];
    for p in pulls.iter() {
        c.add_pull(
            "test/repo",
            c.github.fetch_pull("test/repo", p.number),
            false,
        )
        .await
        .unwrap();
    }

    c.github.test_update_pull(
        "test/repo",
        2,
        &["wiki/Article/en.md", "wiki/Other_article/en.md"],
    );
    c.add_pull("test/repo", c.github.fetch_pull("test/repo", 2), false)
        .await
        .unwrap();

    assert!(c.memory.conflicts("test/repo").get(&2).is_none());
    assert_eq!(
        c.memory.conflicts("test/repo").get(&1).unwrap(),
        &vec![Conflict::new_original_change(
            1,
            2,
            pull_link("test/repo", 2),
            vec![
                "wiki/Article/en.md".to_string(),
                "wiki/Other_article/en.md".to_string()
            ]
        )]
    );
}

#[tokio::test]
async fn test_new_original_change_conflict_double_update() {
    let c = make_controller(true).await;
    let pulls = [
        c.github.test_add_pull("test/repo", &["wiki/Article/ru.md"]),
        c.github.test_add_pull("test/repo", &["wiki/Article/en.md"]),
    ];
    for p in pulls.iter() {
        c.add_pull(
            "test/repo",
            c.github.fetch_pull("test/repo", p.number),
            false,
        )
        .await
        .unwrap();
    }

    c.github.test_update_pull(
        "test/repo",
        1,
        &["wiki/Article/ru.md", "wiki/Other_article/ru.md"],
    );
    c.add_pull("test/repo", c.github.fetch_pull("test/repo", 1), false)
        .await
        .unwrap();
    c.github.test_update_pull(
        "test/repo",
        2,
        &["wiki/Article/en.md", "wiki/Other_article/en.md"],
    );
    c.add_pull("test/repo", c.github.fetch_pull("test/repo", 2), false)
        .await
        .unwrap();

    assert!(c.memory.conflicts("test/repo").get(&2).is_none());
    assert_eq!(
        c.memory.conflicts("test/repo").get(&1).unwrap(),
        &vec![Conflict::new_original_change(
            1,
            2,
            pull_link("test/repo", 2),
            vec![
                "wiki/Article/en.md".to_string(),
                "wiki/Other_article/en.md".to_string()
            ]
        )]
    );
}

#[tokio::test]
async fn test_existing_original_change_conflict_update_no_extra_files() {
    let c = make_controller(true).await;
    let pulls = [
        c.github.test_add_pull("test/repo", &["wiki/Article/en.md"]),
        c.github.test_add_pull("test/repo", &["wiki/Article/ru.md"]),
    ];
    for p in pulls.iter() {
        c.add_pull(
            "test/repo",
            c.github.fetch_pull("test/repo", p.number),
            false,
        )
        .await
        .unwrap();
    }

    c.github.test_update_pull(
        "test/repo",
        1,
        &["wiki/Article/en.md", "wiki/Other_article/en.md"],
    );
    c.add_pull("test/repo", c.github.fetch_pull("test/repo", 1), false)
        .await
        .unwrap();

    assert!(c.memory.conflicts("test/repo").get(&1).is_none());
    assert_eq!(
        c.memory.conflicts("test/repo").get(&2).unwrap(),
        &vec![Conflict::existing_original_change(
            2,
            1,
            pull_link("test/repo", 1),
            vec!["wiki/Article/en.md".to_string()]
        )]
    );
}

#[tokio::test]
async fn test_existing_original_change_conflict_update_recognized() {
    let c = make_controller(true).await;
    let pulls = [
        c.github.test_add_pull(
            "test/repo",
            &["wiki/Article/en.md", "wiki/Other_article/en.md"],
        ),
        c.github.test_add_pull("test/repo", &["wiki/Article/ru.md"]),
    ];
    for p in pulls.iter() {
        c.add_pull(
            "test/repo",
            c.github.fetch_pull("test/repo", p.number),
            false,
        )
        .await
        .unwrap();
    }

    c.github.test_update_pull(
        "test/repo",
        2,
        &["wiki/Article/ru.md", "wiki/Other_article/ru.md"],
    );
    c.add_pull("test/repo", c.github.fetch_pull("test/repo", 2), false)
        .await
        .unwrap();

    assert!(c.memory.conflicts("test/repo").get(&1).is_none());
    assert_eq!(
        c.memory.conflicts("test/repo").get(&2).unwrap(),
        &vec![Conflict::existing_original_change(
            2,
            1,
            pull_link("test/repo", 1),
            vec![
                "wiki/Article/ru.md".to_string(),
                "wiki/Other_article/ru.md".to_string()
            ]
        )]
    );
}

#[tokio::test]
async fn test_three_conflicts_at_once() {
    let c = make_controller(true).await;
    let pulls = [
        c.github.test_add_pull("test/repo", &["wiki/Article/en.md"]),
        c.github
            .test_add_pull("test/repo", &["wiki/Other_article/ru.md"]),
        c.github.test_add_pull(
            "test/repo",
            &[
                "wiki/Article/ru.md",
                "wiki/Other_article/ru.md",
                "wiki/Different_article/ru.md",
            ],
        ),
        c.github
            .test_add_pull("test/repo", &["wiki/Different_article/en.md"]),
    ];
    for p in pulls.iter() {
        c.add_pull(
            "test/repo",
            c.github.fetch_pull("test/repo", p.number),
            false,
        )
        .await
        .unwrap();
    }

    assert_eq!(
        c.memory.conflicts("test/repo").get(&3).unwrap(),
        &vec![
            Conflict::existing_original_change(
                3,
                1,
                pull_link("test/repo", 1),
                vec!["wiki/Article/ru.md".to_string(),]
            ),
            Conflict::existing_change(
                3,
                2,
                pull_link("test/repo", 2),
                vec!["wiki/Other_article/ru.md".to_string(),]
            ),
            Conflict::new_original_change(
                3,
                4,
                pull_link("test/repo", 4),
                vec!["wiki/Different_article/en.md".to_string(),]
            )
        ]
    );
}

#[tokio::test]
async fn test_no_conflict_no_comment() {
    let c = make_controller(true).await;
    let p1 = c.github.test_add_pull("test/repo", &["wiki/Article/en.md"]);
    c.add_pull(
        "test/repo",
        c.github.fetch_pull("test/repo", p1.number),
        true,
    )
    .await
    .unwrap();
    let comments = c
        .github
        .list_comments("test/repo", p1.number)
        .await
        .unwrap();
    assert!(comments.is_empty());
}

#[tokio::test]
async fn test_one_conflict_one_comment() {
    let c = make_controller(true).await;
    let p1 = c.github.test_add_pull("test/repo", &["wiki/Article/en.md"]);
    let p2 = c.github.test_add_pull("test/repo", &["wiki/Article/en.md"]);

    c.add_pull(
        "test/repo",
        c.github.fetch_pull("test/repo", p1.number),
        true,
    )
    .await
    .unwrap();
    c.add_pull(
        "test/repo",
        c.github.fetch_pull("test/repo", p2.number),
        true,
    )
    .await
    .unwrap();

    let first_pull_comments = c
        .github
        .list_comments("test/repo", p1.number)
        .await
        .unwrap();
    assert!(first_pull_comments.is_empty());

    let second_pull_comments = c
        .github
        .list_comments("test/repo", p2.number)
        .await
        .unwrap();
    assert_eq!(second_pull_comments.len(), 1);
}

#[tokio::test]
async fn test_one_conflict_one_valid_header() {
    let c = make_controller(true).await;
    let pulls = [
        c.github.test_add_pull("test/repo", &["wiki/Article/en.md"]),
        c.github.test_add_pull("test/repo", &["wiki/Article/en.md"]),
    ];
    for p in pulls.iter() {
        c.add_pull(
            "test/repo",
            c.github.fetch_pull("test/repo", p.number),
            true,
        )
        .await
        .unwrap();
    }

    let second_pull_comments = c
        .github
        .list_comments("test/repo", pulls[1].number)
        .await
        .unwrap();
    let header = CommentHeader::from_comment(&second_pull_comments.first().unwrap().body).unwrap();
    assert_eq!(
        header,
        CommentHeader {
            pull_number: 1,
            conflict_type: ConflictType::ExistingChange,
        }
    );
}

#[tokio::test]
async fn test_one_pull_and_conflict_one_comment() {
    let c = make_controller(true).await;
    let pulls = [
        c.github.test_add_pull("test/repo", &["wiki/Article/en.md"]),
        c.github.test_add_pull(
            "test/repo",
            &["wiki/Article/en.md", "wiki/Other_article/en.md"],
        ),
    ];
    for p in pulls.iter() {
        c.add_pull(
            "test/repo",
            c.github.fetch_pull("test/repo", p.number),
            true,
        )
        .await
        .unwrap();
    }

    c.github.test_update_pull(
        "test/repo",
        pulls[0].number,
        &["wiki/Article/en.md", "wiki/Other_article/en.md"],
    );
    c.add_pull(
        "test/repo",
        c.github.fetch_pull("test/repo", pulls[0].number),
        true,
    )
    .await
    .unwrap();

    c.github
        .test_update_pull("test/repo", pulls[0].number, &["wiki/Other_article/en.md"]);
    c.add_pull(
        "test/repo",
        c.github.fetch_pull("test/repo", pulls[0].number),
        true,
    )
    .await
    .unwrap();

    let second_pull_comments = c
        .github
        .list_comments("test/repo", pulls[1].number)
        .await
        .unwrap();
    assert_eq!(second_pull_comments.len(), 1);
}

#[tokio::test]
async fn test_one_pull_and_conflict_one_comment_updated() {
    let c = make_controller(true).await;
    let pulls = [
        c.github.test_add_pull("test/repo", &["wiki/Article/en.md"]),
        c.github.test_add_pull(
            "test/repo",
            &["wiki/Article/en.md", "wiki/Other_article/en.md"],
        ),
    ];
    for p in pulls.iter() {
        c.add_pull(
            "test/repo",
            c.github.fetch_pull("test/repo", p.number),
            true,
        )
        .await
        .unwrap();
    }

    c.github.test_update_pull(
        "test/repo",
        pulls[0].number,
        &["wiki/Article/en.md", "wiki/Other_article/en.md"],
    );
    c.add_pull(
        "test/repo",
        c.github.fetch_pull("test/repo", pulls[0].number),
        true,
    )
    .await
    .unwrap();

    c.github
        .test_update_pull("test/repo", pulls[0].number, &["wiki/Other_article/en.md"]);
    c.add_pull(
        "test/repo",
        c.github.fetch_pull("test/repo", pulls[0].number),
        true,
    )
    .await
    .unwrap();

    let second_pull_comments = c
        .github
        .list_comments("test/repo", pulls[1].number)
        .await
        .unwrap();
    let only_comment = &second_pull_comments.first().unwrap().body;
    let header = CommentHeader::from_comment(&only_comment).unwrap();

    assert_eq!(
        header,
        CommentHeader {
            pull_number: 1,
            conflict_type: ConflictType::ExistingChange,
        }
    );
    assert!(only_comment.contains("wiki/Other_article/en.md"));
    assert!(!only_comment.contains("wiki/Article/en.md"));
}

#[tokio::test]
async fn test_post_comment_per_pull_and_conflict_combination() {
    let c = make_controller(true).await;
    let pulls = [
        c.github
            .test_add_pull("test/repo", &["wiki/Article/en.md", "wiki/Article_2/ru.md"]),
        c.github
            .test_add_pull("test/repo", &["wiki/Other_article/en.md"]),
        c.github.test_add_pull(
            "test/repo",
            &[
                "wiki/Article/ru.md",       // ExistingOriginalChange (1)
                "wiki/Article_2/ru.md",     // ExistingChange (1)
                "wiki/Other_article/en.md", // ExistingChange (2)
                "wiki/New_article/ru.md",   // NewOriginalChange (4)
            ],
        ),
        c.github
            .test_add_pull("test/repo", &["wiki/New_article/en.md"]),
    ];
    for p in pulls.iter() {
        c.add_pull(
            "test/repo",
            c.github.fetch_pull("test/repo", p.number),
            true,
        )
        .await
        .unwrap();
    }

    let third_pull_comments = c
        .github
        .list_comments("test/repo", pulls[2].number)
        .await
        .unwrap();
    assert_eq!(third_pull_comments.len(), 4);
    let mut headers: Vec<_> = third_pull_comments
        .into_iter()
        .map(|c| CommentHeader::from_comment(&c.body).unwrap())
        .collect();
    headers.sort();

    let mut expected = [
        CommentHeader {
            pull_number: 1,
            conflict_type: ConflictType::ExistingOriginalChange,
        },
        CommentHeader {
            pull_number: 1,
            conflict_type: ConflictType::ExistingChange,
        },
        CommentHeader {
            pull_number: 2,
            conflict_type: ConflictType::ExistingChange,
        },
        CommentHeader {
            pull_number: 4,
            conflict_type: ConflictType::NewOriginalChange,
        },
    ];
    expected.sort();
    assert_eq!(headers, expected);
}

#[tokio::test]
async fn test_closed_pull_is_removed() {
    let c = make_controller(true).await;
    let pull = c
        .github
        .test_add_pull("test/repo", &["wiki/Article/en.md", "wiki/Article_2/ru.md"]);
    c.add_pull("test/repo", pull, true).await.unwrap();

    c.remove_pull("test/repo", c.github.fetch_pull("test/repo", 1));
    assert!(c.memory.pulls("test/repo").unwrap().is_empty());
}

#[tokio::test]
async fn test_closed_pull_conflicts_removed() {
    let c = make_controller(true).await;
    let pulls = [
        c.github
            .test_add_pull("test/repo", &["wiki/Article/en.md", "wiki/Article_2/ru.md"]),
        c.github
            .test_add_pull("test/repo", &["wiki/Other_article/en.md"]),
        c.github.test_add_pull(
            "test/repo",
            &["wiki/Article/ru.md", "wiki/Other_article/ru.md"],
        ),
    ];
    for p in pulls.iter() {
        c.add_pull(
            "test/repo",
            c.github.fetch_pull("test/repo", p.number),
            true,
        )
        .await
        .unwrap();
    }

    c.remove_pull("test/repo", c.github.fetch_pull("test/repo", 3));
    assert!(c.memory.conflicts("test/repo").get(&3).is_none());
}

#[tokio::test]
async fn test_closed_pull_related_conflicts_removed() {
    let c = make_controller(true).await;
    let pulls = [
        c.github.test_add_pull("test/repo", &["wiki/Article/en.md"]),
        c.github.test_add_pull("test/repo", &["wiki/Article/ru.md"]),
        c.github.test_add_pull("test/repo", &["wiki/Article/cz.md"]),
        c.github.test_add_pull("test/repo", &["wiki/Article/zh.md"]),
    ];
    for p in pulls.iter() {
        c.add_pull(
            "test/repo",
            c.github.fetch_pull("test/repo", p.number),
            true,
        )
        .await
        .unwrap();
    }

    c.remove_pull("test/repo", c.github.fetch_pull("test/repo", 1));
    for p in pulls.iter().skip(1) {
        let conflicts = c.memory.conflicts("test/repo");
        let related_conflicts: Vec<_> = conflicts
            .get(&p.number)
            .unwrap()
            .iter()
            .filter(|c| c.original == p.number || c.trigger == p.number)
            .collect();
        assert!(related_conflicts.is_empty());
    }
}
