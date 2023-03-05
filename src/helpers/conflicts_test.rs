use super::*;

use crate::test::{self, pull_link};

#[test]
fn conflict_to_markdown() {
    let c1 = Conflict::existing_change(
        1,
        2,
        pull_link("test/repo", 2),
        vec!["wiki/Ranking_criteria/en.md".to_string()],
    );
    assert_eq!(
        c1.to_markdown(),
        format!(
            r#"<!--
pull_number: 2
conflict_type: ExistingChange
-->
{}
- https://github.com/test/repo/pull/2, files:
  ```
  wiki/Ranking_criteria/en.md
  ```"#,
            comments::EXISTING_CHANGE_TEMPLATE
        )
    );

    let c2 = Conflict::new_original_change(
        2,
        3,
        pull_link("test/repo", 3),
        vec!["wiki/Ranking_criteria/en.md".to_string(); 11],
    );
    assert_eq!(
        c2.to_markdown(),
        format!(
            r#"<!--
pull_number: 3
conflict_type: NewOriginalChange
-->
{}
- https://github.com/test/repo/pull/3 (>10 files)"#,
            comments::NEW_ORIGINAL_CHANGE_TEMPLATE
        )
    );
}

#[test]
fn article_basic() {
    let original = Article::from_file_path("wiki/Article/en.md");
    assert!(original.is_original());
    assert!(!original.is_translation());
    assert_eq!(original.language, "en");
    assert_eq!(original.path, "wiki/Article");
    assert_eq!(original.file_path(), "wiki/Article/en.md");

    let translation = Article::from_file_path("wiki/Article/ko.md");
    assert!(!translation.is_original());
    assert!(translation.is_translation());
    assert_eq!(translation.language, "ko");
    assert_eq!(translation.path, "wiki/Article");
    assert_eq!(translation.file_path(), "wiki/Article/ko.md");

    assert_ne!(original, translation);
}

#[test]
fn different_paths_no_conflict() {
    let existing_pull = test::make_pull(1, &["wiki/First_article/en.md"]);
    let new_pull = test::make_pull(2, &["wiki/Second_article/en.md"]);
    assert!(compare_pulls(&new_pull, &existing_pull).is_empty());
}

#[test]
fn no_markdown_no_conflict() {
    let existing_pull = test::make_pull(1, &["wiki/First_article/img/test.png"]);
    let new_pull = test::make_pull(2, &["wiki/First_article/img/test.png"]);
    assert!(compare_pulls(&new_pull, &existing_pull).is_empty());
}

#[test]
fn single_file_existing_change() {
    let existing_pull = test::make_pull(1, &["wiki/Article/en.md"]);
    let new_pull = test::make_pull(2, &["wiki/Article/en.md"]);

    assert_eq!(
        compare_pulls(&new_pull, &existing_pull),
        vec![Conflict::existing_change(
            2,
            1,
            pull_link("test/repo", 1),
            vec!["wiki/Article/en.md".to_string()],
        )]
    );
}

#[test]
fn multiple_files_existing_change() {
    let existing_pull = test::make_pull(
        1,
        &[
            "wiki/Article/en.md",
            "wiki/Ranking_criteria/en.md",
            "wiki/Article/img/test.png",
            "wiki/Unrelated_article/ru.md",
        ],
    );
    let new_pull = test::make_pull(
        2,
        &[
            "wiki/Ranking_criteria/en.md",
            "wiki/Article/en.md",
            "wiki/Some_other_article/en.md",
            "wiki/Test_article/en.md",
        ],
    );

    assert_eq!(
        compare_pulls(&new_pull, &existing_pull),
        vec![Conflict::existing_change(
            2,
            1,
            pull_link("test/repo", 1),
            vec![
                "wiki/Article/en.md".to_string(),
                "wiki/Ranking_criteria/en.md".to_string(),
            ]
        )]
    );
    assert_eq!(
        compare_pulls(&existing_pull, &new_pull),
        vec![Conflict::existing_change(
            1,
            2,
            pull_link("test/repo", 2),
            vec![
                "wiki/Article/en.md".to_string(),
                "wiki/Ranking_criteria/en.md".to_string(),
            ]
        )]
    );
}

#[test]
fn existing_translation_new_original_change() {
    let existing_pull = test::make_pull(1, &["wiki/Article/ru.md"]);
    let new_pull = test::make_pull(2, &["wiki/Article/en.md"]);

    assert_eq!(
        compare_pulls(&new_pull, &existing_pull),
        vec![Conflict::new_original_change(
            1,
            2,
            pull_link("test/repo", 2),
            vec!["wiki/Article/en.md".to_string(),],
        )]
    );
}

#[test]
fn new_translation_existing_original_change() {
    let existing_pull = test::make_pull(1, &["wiki/Article/en.md"]);
    let new_pull = test::make_pull(2, &["wiki/Article/ru.md"]);

    assert_eq!(
        compare_pulls(&new_pull, &existing_pull),
        vec![Conflict::existing_original_change(
            2,
            1,
            pull_link("test/repo", 1),
            vec!["wiki/Article/ru.md".to_string(),],
        )]
    );
}
