use super::*;
use crate::github::GitHubInterface;
use crate::helpers::{comments::CommentHeader, conflicts::ConflictType};

#[tokio::test]
async fn test_no_conflict_no_comment() {
    let c = new_controller(true).await;
    let p1 = c.github.test_add_pull("test/repo", &["wiki/Article/en.md"]);
    c.upsert_pull(
        "test/repo",
        c.github.fetch_pull("test/repo", p1.number),
        true,
    )
    .await
    .unwrap();
    let comments = c
        .github
        .read_comments("test/repo", p1.number)
        .await
        .unwrap();
    assert!(comments.is_empty());
}

#[tokio::test]
async fn test_one_conflict_one_comment() {
    let c = new_controller(true).await;
    let p1 = c.github.test_add_pull("test/repo", &["wiki/Article/en.md"]);
    let p2 = c.github.test_add_pull("test/repo", &["wiki/Article/en.md"]);

    c.upsert_pull(
        "test/repo",
        c.github.fetch_pull("test/repo", p1.number),
        true,
    )
    .await
    .unwrap();
    c.upsert_pull(
        "test/repo",
        c.github.fetch_pull("test/repo", p2.number),
        true,
    )
    .await
    .unwrap();

    let first_pull_comments = c
        .github
        .read_comments("test/repo", p1.number)
        .await
        .unwrap();
    assert!(first_pull_comments.is_empty());

    let second_pull_comments = c
        .github
        .read_comments("test/repo", p2.number)
        .await
        .unwrap();
    assert_eq!(second_pull_comments.len(), 1);
}

#[tokio::test]
async fn test_one_conflict_one_valid_header() {
    let c = new_controller(true).await;
    let pulls = [
        c.github.test_add_pull("test/repo", &["wiki/Article/en.md"]),
        c.github.test_add_pull("test/repo", &["wiki/Article/en.md"]),
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

    let second_pull_comments = c
        .github
        .read_comments("test/repo", pulls[1].number)
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
    let c = new_controller(true).await;
    let pulls = [
        c.github.test_add_pull("test/repo", &["wiki/Article/en.md"]),
        c.github.test_add_pull(
            "test/repo",
            &["wiki/Article/en.md", "wiki/Other_article/en.md"],
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

    c.github.test_update_pull(
        "test/repo",
        pulls[0].number,
        &["wiki/Article/en.md", "wiki/Other_article/en.md"],
    );
    c.upsert_pull(
        "test/repo",
        c.github.fetch_pull("test/repo", pulls[0].number),
        true,
    )
    .await
    .unwrap();

    c.github
        .test_update_pull("test/repo", pulls[0].number, &["wiki/Other_article/en.md"]);
    c.upsert_pull(
        "test/repo",
        c.github.fetch_pull("test/repo", pulls[0].number),
        true,
    )
    .await
    .unwrap();

    let second_pull_comments = c
        .github
        .read_comments("test/repo", pulls[1].number)
        .await
        .unwrap();
    assert_eq!(second_pull_comments.len(), 1);
}

#[tokio::test]
async fn test_one_pull_and_conflict_one_comment_updated() {
    let c = new_controller(true).await;
    let pulls = [
        c.github.test_add_pull("test/repo", &["wiki/Article/en.md"]),
        c.github.test_add_pull(
            "test/repo",
            &["wiki/Article/en.md", "wiki/Other_article/en.md"],
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

    c.github.test_update_pull(
        "test/repo",
        pulls[0].number,
        &["wiki/Article/en.md", "wiki/Other_article/en.md"],
    );
    c.upsert_pull(
        "test/repo",
        c.github.fetch_pull("test/repo", pulls[0].number),
        true,
    )
    .await
    .unwrap();

    c.github
        .test_update_pull("test/repo", pulls[0].number, &["wiki/Other_article/en.md"]);
    c.upsert_pull(
        "test/repo",
        c.github.fetch_pull("test/repo", pulls[0].number),
        true,
    )
    .await
    .unwrap();

    let second_pull_comments = c
        .github
        .read_comments("test/repo", pulls[1].number)
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
    let c = new_controller(true).await;
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
        c.upsert_pull(
            "test/repo",
            c.github.fetch_pull("test/repo", p.number),
            true,
        )
        .await
        .unwrap();
    }

    let third_pull_comments = c
        .github
        .read_comments("test/repo", pulls[2].number)
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
    let c = new_controller(true).await;
    let pulls = [
        c.github
            .test_add_pull("test/repo", &["wiki/Article/en.md", "wiki/Article_2/ru.md"]),
        c.github.test_add_pull("test/repo", &["wiki/Article/en.md"]),
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

    c.github
        .test_update_pull("test/repo", 1, &["wiki/Article_2/ru.md"]);
    c.upsert_pull("test/repo", c.github.fetch_pull("test/repo", 1), true)
        .await
        .unwrap();
    assert!(c
        .github
        .read_comments("test/repo", 2)
        .await
        .unwrap()
        .is_empty())
}

#[tokio::test]
async fn test_only_target_comment_is_removed() {
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
            true,
        )
        .await
        .unwrap();
    }

    assert_eq!(
        c.github.read_comments("test/repo", 2).await.unwrap().len(),
        2
    );

    c.github
        .test_update_pull("test/repo", 1, &["wiki/Article/Other_article/en.md"]);
    c.upsert_pull("test/repo", c.github.fetch_pull("test/repo", 1), true)
        .await
        .unwrap();

    let comments = c.github.read_comments("test/repo", 2).await.unwrap();
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
    let c = new_controller(true).await;
    let pulls = [
        c.github.test_add_pull("test/repo", &["wiki/Article/ru.md"]),
        c.github.test_add_pull("test/repo", &["wiki/Article/ru.md"]),
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

    assert_eq!(
        c.github.read_comments("test/repo", 2).await.unwrap().len(),
        1
    );

    let first_conflict_created_at = chrono::Utc::now();

    c.github
        .test_update_pull("test/repo", 1, &["wiki/Article/Other_article/en.md"]);
    c.upsert_pull("test/repo", c.github.fetch_pull("test/repo", 1), true)
        .await
        .unwrap();

    assert!(c
        .github
        .read_comments("test/repo", 2)
        .await
        .unwrap()
        .is_empty());

    c.github
        .test_update_pull("test/repo", 1, &["wiki/Article/ru.md"]);
    c.upsert_pull("test/repo", c.github.fetch_pull("test/repo", 1), true)
        .await
        .unwrap();

    let comments = c.github.read_comments("test/repo", 1).await.unwrap();
    assert_eq!(comments.first().unwrap().id, 2);
    assert!(comments.first().unwrap().created_at > first_conflict_created_at);
}
