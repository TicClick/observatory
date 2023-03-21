use super::*;

use crate::helpers::conflicts::Conflict;
use crate::test::{self, pull_link};

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
async fn test_simple_overlap_originals() {
    let c = make_controller(true).await;
    let pulls = [
        c.github.test_add_pull("test/repo", &["wiki/Article/en.md"]),
        c.github.test_add_pull("test/repo", &["wiki/Article/en.md"]),
    ];
    for p in pulls {
        c.add_pull("test/repo", p, false).await.unwrap();
    }

    assert!(c.conflicts.by_trigger("test/repo", 1).is_empty());
    assert_eq!(
        &c.conflicts.by_trigger("test/repo", 2),
        &vec![Conflict::overlap(
            2,
            1,
            pull_link("test/repo", 1),
            vec!["wiki/Article/en.md".to_string()]
        )]
    );
}

#[tokio::test]
async fn test_simple_overlap_translations() {
    let c = make_controller(true).await;
    let pulls = [
        c.github.test_add_pull("test/repo", &["wiki/Article/ru.md"]),
        c.github.test_add_pull("test/repo", &["wiki/Article/ru.md"]),
    ];
    for p in pulls {
        c.add_pull("test/repo", p, false).await.unwrap();
    }

    assert!(c.conflicts.by_trigger("test/repo", 1).is_empty());
    assert_eq!(
        &c.conflicts.by_trigger("test/repo", 2),
        &vec![Conflict::overlap(
            2,
            1,
            pull_link("test/repo", 1),
            vec!["wiki/Article/ru.md".to_string()]
        )]
    );
}

#[tokio::test]
async fn test_different_translations_do_not_overlap() {
    let c = make_controller(true).await;
    let pulls = [
        c.github.test_add_pull("test/repo", &["wiki/Article/ru.md"]),
        c.github.test_add_pull("test/repo", &["wiki/Article/ko.md"]),
    ];
    for p in pulls {
        c.add_pull("test/repo", p, false).await.unwrap();
    }

    assert!(c.conflicts.by_trigger("test/repo", 1).is_empty());
    assert!(c.conflicts.by_trigger("test/repo", 2).is_empty());
}

#[tokio::test]
async fn test_simple_early_incomplete_translation() {
    let c = make_controller(true).await;
    let pulls = [
        c.github.test_add_pull("test/repo", &["wiki/Article/ru.md"]),
        c.github.test_add_pull("test/repo", &["wiki/Article/en.md"]),
    ];
    for p in pulls {
        c.add_pull("test/repo", p, false).await.unwrap();
    }

    assert_eq!(
        &c.conflicts.by_trigger("test/repo", 1),
        &vec![Conflict::incomplete_translation(
            1,
            2,
            pull_link("test/repo", 2),
            vec!["wiki/Article/en.md".to_string()]
        )]
    );
    assert!(c.conflicts.by_trigger("test/repo", 2).is_empty());
}

#[tokio::test]
async fn test_simple_late_incomplete_translation() {
    let c = make_controller(true).await;
    let pulls = [
        c.github.test_add_pull("test/repo", &["wiki/Article/en.md"]),
        c.github.test_add_pull("test/repo", &["wiki/Article/ru.md"]),
    ];
    for p in pulls {
        c.add_pull("test/repo", p, false).await.unwrap();
    }

    assert!(&c.conflicts.by_trigger("test/repo", 1).is_empty());
    assert_eq!(
        &c.conflicts.by_trigger("test/repo", 2),
        &vec![Conflict::incomplete_translation(
            2,
            1,
            pull_link("test/repo", 1),
            vec!["wiki/Article/en.md".to_string()]
        )]
    );
}

#[tokio::test]
async fn test_multiple_overlapping_changes() {
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

    let overlap = |trigger, original| {
        Conflict::overlap(
            trigger,
            original,
            pull_link("test/repo", original),
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

    let incomplete_translation = |trigger, original| {
        Conflict::incomplete_translation(
            trigger,
            original,
            pull_link("test/repo", original),
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

    let incomplete_translation_conflict = |trigger, original| {
        Conflict::incomplete_translation(
            trigger,
            original,
            pull_link("test/repo", original),
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

    assert!(&c.conflicts.by_trigger("test/repo", 1).is_empty());
    assert_eq!(
        &c.conflicts.by_trigger("test/repo", 2),
        &vec![Conflict::overlap(
            2,
            1,
            pull_link("test/repo", 1),
            vec!["wiki/Article/ru.md".to_string()]
        )]
    );
}

#[tokio::test]
async fn test_overlap_file_set_update_in_trigger_recognized() {
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

    assert!(&c.conflicts.by_trigger("test/repo", 1).is_empty());
    assert_eq!(
        &c.conflicts.by_trigger("test/repo", 2),
        &vec![Conflict::overlap(
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
async fn test_overlap_double_update_recognized() {
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

    assert!(&c.conflicts.by_trigger("test/repo", 1).is_empty());
    assert_eq!(
        &c.conflicts.by_trigger("test/repo", 2),
        &vec![Conflict::overlap(
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
async fn test_early_incomplete_translation_update_no_unrelated_files() {
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

    assert!(&c.conflicts.by_trigger("test/repo", 2).is_empty());
    assert_eq!(
        &c.conflicts.by_trigger("test/repo", 1),
        &vec![Conflict::incomplete_translation(
            1,
            2,
            pull_link("test/repo", 2),
            vec!["wiki/Article/en.md".to_string()]
        )]
    );
}

#[tokio::test]
async fn test_incomplete_translation_original_update_recognized() {
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

    assert!(&c.conflicts.by_trigger("test/repo", 2).is_empty());
    assert_eq!(
        &c.conflicts.by_trigger("test/repo", 1),
        &vec![Conflict::incomplete_translation(
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
async fn test_incomplete_translation_double_update() {
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

    assert!(&c.conflicts.by_trigger("test/repo", 2).is_empty());
    assert_eq!(
        &c.conflicts.by_trigger("test/repo", 1),
        &vec![Conflict::incomplete_translation(
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
async fn test_late_incomplete_translation_update_no_extra_files() {
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

    assert!(&c.conflicts.by_trigger("test/repo", 1).is_empty());
    assert_eq!(
        &c.conflicts.by_trigger("test/repo", 2),
        &vec![Conflict::incomplete_translation(
            2,
            1,
            pull_link("test/repo", 1),
            vec!["wiki/Article/en.md".to_string()]
        )]
    );
}

#[tokio::test]
async fn test_incomplete_translation_update_recognized() {
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

    assert!(&c.conflicts.by_trigger("test/repo", 1).is_empty());
    assert_eq!(
        &c.conflicts.by_trigger("test/repo", 2),
        &vec![Conflict::incomplete_translation(
            2,
            1,
            pull_link("test/repo", 1),
            vec![
                "wiki/Article/en.md".to_string(),
                "wiki/Other_article/en.md".to_string()
            ]
        )]
    );
}

#[tokio::test]
async fn test_outdated_translation_produces_single_conflict() {
    let c = make_controller(true).await;
    let pulls = [
        c.github
            .test_add_pull("test/repo", &["wiki/Article/en.md", "wiki/Article/ru.md"]),
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

    assert!(&c.conflicts.by_trigger("test/repo", 1).is_empty());
    assert_eq!(
        &c.conflicts.by_trigger("test/repo", 2),
        &vec![Conflict::incomplete_translation(
            2,
            1,
            pull_link("test/repo", 1),
            vec!["wiki/Article/en.md".to_string(),]
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
        &c.conflicts.by_trigger("test/repo", 3),
        &vec![
            Conflict::overlap(
                3,
                2,
                pull_link("test/repo", 2),
                vec!["wiki/Other_article/ru.md".to_string(),]
            ),
            Conflict::incomplete_translation(
                3,
                1,
                pull_link("test/repo", 1),
                vec!["wiki/Article/en.md".to_string(),]
            ),
            Conflict::incomplete_translation(
                3,
                4,
                pull_link("test/repo", 4),
                vec!["wiki/Different_article/en.md".to_string(),]
            ),
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
            conflict_type: ConflictType::Overlap,
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
            conflict_type: ConflictType::Overlap,
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
            conflict_type: ConflictType::IncompleteTranslation,
        },
        CommentHeader {
            pull_number: 1,
            conflict_type: ConflictType::Overlap,
        },
        CommentHeader {
            pull_number: 2,
            conflict_type: ConflictType::Overlap,
        },
        CommentHeader {
            pull_number: 4,
            conflict_type: ConflictType::IncompleteTranslation,
        },
    ];
    expected.sort();
    assert_eq!(headers, expected);
}

#[tokio::test]
async fn test_obsolete_comment_is_removed() {
    let c = make_controller(true).await;
    let pulls = [
        c.github
            .test_add_pull("test/repo", &["wiki/Article/en.md", "wiki/Article_2/ru.md"]),
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

    c.github
        .test_update_pull("test/repo", 1, &["wiki/Article_2/ru.md"]);
    c.add_pull("test/repo", c.github.fetch_pull("test/repo", 1), true)
        .await
        .unwrap();
    assert!(c
        .github
        .list_comments("test/repo", 2)
        .await
        .unwrap()
        .is_empty())
}

#[tokio::test]
async fn test_only_target_comment_is_removed() {
    let c = make_controller(true).await;
    let pulls = [
        c.github.test_add_pull(
            "test/repo",
            &["wiki/Article/ru.md", "wiki/Article/Other_article/en.md"],
        ),
        c.github.test_add_pull(
            "test/repo",
            &["wiki/Article/ru.md", "wiki/Article/Other_article/ru.md"],
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

    assert_eq!(
        c.github.list_comments("test/repo", 2).await.unwrap().len(),
        2
    );

    c.github
        .test_update_pull("test/repo", 1, &["wiki/Article/Other_article/en.md"]);
    c.add_pull("test/repo", c.github.fetch_pull("test/repo", 1), true)
        .await
        .unwrap();

    let comments = c.github.list_comments("test/repo", 2).await.unwrap();
    assert_eq!(comments.len(), 1);
    let h = CommentHeader::from_comment(&comments.first().unwrap().body).unwrap();
    assert_eq!(
        h,
        CommentHeader {
            pull_number: 1,
            conflict_type: ConflictType::IncompleteTranslation
        }
    );
}

#[tokio::test]
async fn test_new_comment_is_posted_after_removal_in_different_pull() {
    let c = make_controller(true).await;
    let pulls = [
        c.github.test_add_pull("test/repo", &["wiki/Article/ru.md"]),
        c.github.test_add_pull("test/repo", &["wiki/Article/ru.md"]),
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

    assert_eq!(
        c.github.list_comments("test/repo", 2).await.unwrap().len(),
        1
    );

    let first_conflict_created_at = chrono::Utc::now();

    c.github
        .test_update_pull("test/repo", 1, &["wiki/Article/Other_article/en.md"]);
    c.add_pull("test/repo", c.github.fetch_pull("test/repo", 1), true)
        .await
        .unwrap();

    assert!(c
        .github
        .list_comments("test/repo", 2)
        .await
        .unwrap()
        .is_empty());

    c.github
        .test_update_pull("test/repo", 1, &["wiki/Article/ru.md"]);
    c.add_pull("test/repo", c.github.fetch_pull("test/repo", 1), true)
        .await
        .unwrap();

    let comments = c.github.list_comments("test/repo", 1).await.unwrap();
    assert_eq!(comments.first().unwrap().id, 2);
    assert!(comments.first().unwrap().created_at > first_conflict_created_at);
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
    assert!(&c.conflicts.by_trigger("test/repo", 3).is_empty());
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
        assert!(c.conflicts.by_original("test/repo", p.number).is_empty());
        assert!(c.conflicts.by_trigger("test/repo", p.number).is_empty());
    }
}

#[tokio::test]
async fn test_obsolete_conflict_removed() {
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

    c.github
        .test_update_pull("test/repo", 1, &["wiki/Other_article/ru.md"]);
    c.add_pull("test/repo", c.github.fetch_pull("test/repo", 1), false)
        .await
        .unwrap();

    assert!(c.conflicts.by_original("test/repo", 1).is_empty());
    assert!(c.conflicts.by_original("test/repo", 2).is_empty());
}

#[tokio::test]
async fn test_only_obsolete_conflict_is_removed_overlap() {
    let c = make_controller(true).await;
    let pulls = [
        c.github.test_add_pull(
            "test/repo",
            &["wiki/Article/ru.md", "wiki/Article/Other_article/en.md"],
        ),
        c.github.test_add_pull(
            "test/repo",
            &["wiki/Article/ru.md", "wiki/Article/Other_article/ru.md"],
        ),
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

    c.github
        .test_update_pull("test/repo", 1, &["wiki/Article/Other_article/en.md"]);
    c.add_pull("test/repo", c.github.fetch_pull("test/repo", 1), false)
        .await
        .unwrap();

    assert_eq!(
        &c.conflicts.by_trigger("test/repo", 2),
        &vec![Conflict::incomplete_translation(
            2,
            1,
            pull_link("test/repo", 1),
            vec!["wiki/Article/Other_article/en.md".to_string()]
        )]
    );
}

#[tokio::test]
async fn test_only_obsolete_conflict_is_removed_incomplete_translation() {
    let c = make_controller(true).await;
    let pulls = [
        c.github.test_add_pull(
            "test/repo",
            &["wiki/Article/ru.md", "wiki/Article/Other_article/en.md"],
        ),
        c.github.test_add_pull(
            "test/repo",
            &["wiki/Article/ru.md", "wiki/Article/Other_article/ru.md"],
        ),
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

    c.github
        .test_update_pull("test/repo", 1, &["wiki/Article/ru.md"]);
    c.add_pull("test/repo", c.github.fetch_pull("test/repo", 1), false)
        .await
        .unwrap();

    assert!(c.conflicts.by_trigger("test/repo", 1).is_empty());
    assert_eq!(
        &c.conflicts.by_trigger("test/repo", 2),
        &vec![Conflict::overlap(
            2,
            1,
            pull_link("test/repo", 1),
            vec!["wiki/Article/ru.md".to_string()]
        )]
    );
}
