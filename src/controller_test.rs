use super::*;

use crate::helpers::pulls::Conflict;
use crate::test::{self, pull_link};

async fn make_controller(init: bool) -> Controller<test::DummyGitHubClient> {
    let mut c =
        Controller::<test::DummyGitHubClient>::new("123".to_string(), "private-key".to_string());
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
