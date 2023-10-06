use super::*;

use crate::helpers::conflicts::Conflict;
use crate::test::pull_link;

#[tokio::test]
async fn test_add_pull() {
    let c = new_controller(true).await;
    let mut p = c.github.test_add_pull("test/repo", &["wiki/Article/en.md"]);
    p.diff = None;
    let pn = p.number;

    c.upsert_pull("test/repo", p, false).await.unwrap();

    let m = c.memory.pulls("test/repo");
    assert!(m.is_some());
    assert!(m.unwrap().get(&pn).unwrap().diff.is_some());
}

#[tokio::test]
async fn test_simple_overlap_originals() {
    let c = new_controller(true).await;
    let pulls = [
        c.github.test_add_pull("test/repo", &["wiki/Article/en.md"]),
        c.github.test_add_pull("test/repo", &["wiki/Article/en.md"]),
    ];
    for p in pulls {
        c.upsert_pull("test/repo", p, false).await.unwrap();
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
    let c = new_controller(true).await;
    let pulls = [
        c.github.test_add_pull("test/repo", &["wiki/Article/ru.md"]),
        c.github.test_add_pull("test/repo", &["wiki/Article/ru.md"]),
    ];
    for p in pulls {
        c.upsert_pull("test/repo", p, false).await.unwrap();
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
    let c = new_controller(true).await;
    let pulls = [
        c.github.test_add_pull("test/repo", &["wiki/Article/ru.md"]),
        c.github.test_add_pull("test/repo", &["wiki/Article/ko.md"]),
    ];
    for p in pulls {
        c.upsert_pull("test/repo", p, false).await.unwrap();
    }

    assert!(c.conflicts.by_trigger("test/repo", 1).is_empty());
    assert!(c.conflicts.by_trigger("test/repo", 2).is_empty());
}

#[tokio::test]
async fn test_simple_early_incomplete_translation() {
    let c = new_controller(true).await;
    let pulls = [
        c.github.test_add_pull("test/repo", &["wiki/Article/ru.md"]),
        c.github.test_add_pull("test/repo", &["wiki/Article/en.md"]),
    ];
    for p in pulls {
        c.upsert_pull("test/repo", p, false).await.unwrap();
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
    let c = new_controller(true).await;
    let pulls = [
        c.github.test_add_pull("test/repo", &["wiki/Article/en.md"]),
        c.github.test_add_pull("test/repo", &["wiki/Article/ru.md"]),
    ];
    for p in pulls {
        c.upsert_pull("test/repo", p, false).await.unwrap();
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
    let c = new_controller(true).await;
    let pulls = [
        c.github.test_add_pull("test/repo", &["wiki/Article/en.md"]),
        c.github.test_add_pull("test/repo", &["wiki/Article/en.md"]),
        c.github.test_add_pull("test/repo", &["wiki/Article/en.md"]),
        c.github.test_add_pull("test/repo", &["wiki/Article/en.md"]),
    ];
    for p in pulls {
        c.upsert_pull("test/repo", p, false).await.unwrap();
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
    let c = new_controller(true).await;
    let pulls = [
        c.github.test_add_pull("test/repo", &["wiki/Article/ru.md"]),
        c.github.test_add_pull("test/repo", &["wiki/Article/jp.md"]),
        c.github.test_add_pull("test/repo", &["wiki/Article/ko.md"]),
        c.github.test_add_pull("test/repo", &["wiki/Article/en.md"]),
    ];
    for p in pulls {
        c.upsert_pull("test/repo", p, false).await.unwrap();
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
    let c = new_controller(true).await;
    let pulls = [
        c.github.test_add_pull("test/repo", &["wiki/Article/en.md"]),
        c.github.test_add_pull("test/repo", &["wiki/Article/ru.md"]),
        c.github.test_add_pull("test/repo", &["wiki/Article/jp.md"]),
        c.github.test_add_pull("test/repo", &["wiki/Article/ko.md"]),
    ];
    for p in pulls {
        c.upsert_pull("test/repo", p, false).await.unwrap();
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
    let c = new_controller(true).await;
    let pulls = [
        c.github.test_add_pull("test/repo", &["wiki/Article/ru.md"]),
        c.github.test_add_pull("test/repo", &["wiki/Article/ru.md"]),
    ];
    for p in pulls.iter() {
        c.upsert_pull("test/repo", p.clone(), false).await.unwrap();
    }

    c.github.test_update_pull(
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
            pull_link("test/repo", 1),
            vec!["wiki/Article/ru.md".to_string()]
        )]
    );
}

#[tokio::test]
async fn test_overlap_file_set_update_in_trigger_recognized() {
    let c = new_controller(true).await;
    let pulls = [
        c.github.test_add_pull("test/repo", &["wiki/Article/ru.md"]),
        c.github.test_add_pull(
            "test/repo",
            &["wiki/Article/ru.md", "wiki/Other_article/en.md"],
        ),
    ];
    for p in pulls.iter() {
        c.upsert_pull("test/repo", p.clone(), false).await.unwrap();
    }

    c.github.test_update_pull(
        "test/repo",
        1,
        &["wiki/Article/ru.md", "wiki/Other_article/en.md"],
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
    let c = new_controller(true).await;
    let pulls = [
        c.github.test_add_pull("test/repo", &["wiki/Article/ru.md"]),
        c.github.test_add_pull("test/repo", &["wiki/Article/ru.md"]),
    ];
    for p in pulls.iter() {
        c.upsert_pull(
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
    c.upsert_pull("test/repo", c.github.fetch_pull("test/repo", 1), false)
        .await
        .unwrap();
    c.github.test_update_pull(
        "test/repo",
        2,
        &["wiki/Article/ru.md", "wiki/Other_article/en.md"],
    );
    c.upsert_pull("test/repo", c.github.fetch_pull("test/repo", 2), false)
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
    let c = new_controller(true).await;
    let pulls = [
        c.github.test_add_pull("test/repo", &["wiki/Article/ru.md"]),
        c.github.test_add_pull("test/repo", &["wiki/Article/en.md"]),
    ];
    for p in pulls.iter() {
        c.upsert_pull(
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
    c.upsert_pull("test/repo", c.github.fetch_pull("test/repo", 2), false)
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
    let c = new_controller(true).await;
    let pulls = [
        c.github.test_add_pull(
            "test/repo",
            &["wiki/Article/ru.md", "wiki/Other_article/sv.md"],
        ),
        c.github.test_add_pull("test/repo", &["wiki/Article/en.md"]),
    ];
    for p in pulls.iter() {
        c.upsert_pull(
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
    c.upsert_pull("test/repo", c.github.fetch_pull("test/repo", 2), false)
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
    let c = new_controller(true).await;
    let pulls = [
        c.github.test_add_pull("test/repo", &["wiki/Article/ru.md"]),
        c.github.test_add_pull("test/repo", &["wiki/Article/en.md"]),
    ];
    for p in pulls.iter() {
        c.upsert_pull(
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
    c.upsert_pull("test/repo", c.github.fetch_pull("test/repo", 1), false)
        .await
        .unwrap();
    c.github.test_update_pull(
        "test/repo",
        2,
        &["wiki/Article/en.md", "wiki/Other_article/en.md"],
    );
    c.upsert_pull("test/repo", c.github.fetch_pull("test/repo", 2), false)
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
    let c = new_controller(true).await;
    let pulls = [
        c.github.test_add_pull("test/repo", &["wiki/Article/en.md"]),
        c.github.test_add_pull("test/repo", &["wiki/Article/ru.md"]),
    ];
    for p in pulls.iter() {
        c.upsert_pull(
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
    c.upsert_pull("test/repo", c.github.fetch_pull("test/repo", 1), false)
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
    let c = new_controller(true).await;
    let pulls = [
        c.github.test_add_pull(
            "test/repo",
            &["wiki/Article/en.md", "wiki/Other_article/en.md"],
        ),
        c.github.test_add_pull("test/repo", &["wiki/Article/ru.md"]),
    ];
    for p in pulls.iter() {
        c.upsert_pull(
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
    c.upsert_pull("test/repo", c.github.fetch_pull("test/repo", 2), false)
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
    let c = new_controller(true).await;
    let pulls = [
        c.github
            .test_add_pull("test/repo", &["wiki/Article/en.md", "wiki/Article/ru.md"]),
        c.github.test_add_pull("test/repo", &["wiki/Article/ru.md"]),
    ];
    for p in pulls.iter() {
        c.upsert_pull(
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
    let c = new_controller(true).await;
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
        c.upsert_pull(
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
async fn test_closed_pull_is_removed() {
    let c = new_controller(true).await;
    let pull = c
        .github
        .test_add_pull("test/repo", &["wiki/Article/en.md", "wiki/Article_2/ru.md"]);
    c.upsert_pull("test/repo", pull, true).await.unwrap();

    c.remove_pull("test/repo", c.github.fetch_pull("test/repo", 1));
    assert!(c.memory.pulls("test/repo").unwrap().is_empty());
}

#[tokio::test]
async fn test_closed_pull_conflicts_removed() {
    let c = new_controller(true).await;
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
        c.upsert_pull(
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
    let c = new_controller(true).await;
    let pulls = [
        c.github.test_add_pull("test/repo", &["wiki/Article/en.md"]),
        c.github.test_add_pull("test/repo", &["wiki/Article/ru.md"]),
        c.github.test_add_pull("test/repo", &["wiki/Article/cz.md"]),
        c.github.test_add_pull("test/repo", &["wiki/Article/zh.md"]),
    ];
    for p in pulls.iter() {
        c.upsert_pull(
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
    let c = new_controller(true).await;
    let pulls = [
        c.github.test_add_pull("test/repo", &["wiki/Article/ru.md"]),
        c.github.test_add_pull("test/repo", &["wiki/Article/ru.md"]),
    ];
    for p in pulls.iter() {
        c.upsert_pull(
            "test/repo",
            c.github.fetch_pull("test/repo", p.number),
            false,
        )
        .await
        .unwrap();
    }

    c.github
        .test_update_pull("test/repo", 1, &["wiki/Other_article/ru.md"]);
    c.upsert_pull("test/repo", c.github.fetch_pull("test/repo", 1), false)
        .await
        .unwrap();

    assert!(c.conflicts.by_original("test/repo", 1).is_empty());
    assert!(c.conflicts.by_original("test/repo", 2).is_empty());
}

#[tokio::test]
async fn test_only_obsolete_conflict_is_removed_overlap() {
    let c = new_controller(true).await;
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
        c.upsert_pull(
            "test/repo",
            c.github.fetch_pull("test/repo", p.number),
            false,
        )
        .await
        .unwrap();
    }

    c.github
        .test_update_pull("test/repo", 1, &["wiki/Article/Other_article/en.md"]);
    c.upsert_pull("test/repo", c.github.fetch_pull("test/repo", 1), false)
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
    let c = new_controller(true).await;
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
        c.upsert_pull(
            "test/repo",
            c.github.fetch_pull("test/repo", p.number),
            false,
        )
        .await
        .unwrap();
    }

    c.github
        .test_update_pull("test/repo", 1, &["wiki/Article/ru.md"]);
    c.upsert_pull("test/repo", c.github.fetch_pull("test/repo", 1), false)
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
