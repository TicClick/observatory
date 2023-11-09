use super::*;

use crate::{github, test};

#[test]
fn conflict_to_markdown() {
    let gh = github::GitHub::default();
    let c1 = Conflict::overlap(
        1,
        2,
        gh.pull_url("test/repo", 2),
        vec!["wiki/Ranking_Criteria/en.md".to_string()],
    );
    assert_eq!(
        c1.to_markdown(),
        format!(
            r#"<!--
pull_number: 2
conflict_type: Overlap
-->
{}
- https://github.com/test/repo/pull/2, files:
  - [`wiki/Ranking_Criteria/en.md`](https://github.com/test/repo/pull/2/files#diff-d83e7a1fb8077f937a9a91827c6cb673767a7ebb721e3482bdc146a80802b3d2)"#,
            comments::OVERLAP_TEMPLATE
        )
    );

    let c2 = Conflict::incomplete_translation(
        2,
        3,
        gh.pull_url("test/repo", 3),
        vec!["wiki/Ranking_criteria/en.md".to_string(); 11],
    );
    assert_eq!(
        c2.to_markdown(),
        format!(
            r#"<!--
pull_number: 3
conflict_type: IncompleteTranslation
-->
{}
- https://github.com/test/repo/pull/3 (>10 files)"#,
            comments::INCOMPLETE_TRANSLATION_TEMPLATE
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
    let mut gh = test::GitHubServer::new();

    let existing_pull = gh.make_pull("test/repo-name", &["wiki/First_article/en.md"]);
    let new_pull = gh.make_pull("test/repo-name", &["wiki/Second_article/en.md"]);

    assert!(compare_pulls(&new_pull, &existing_pull).is_empty());
}

#[test]
fn no_markdown_no_conflict() {
    let mut gh = test::GitHubServer::new();

    let existing_pull = gh.make_pull("test/repo-name", &["wiki/First_article/img/test.png"]);
    let new_pull = gh.make_pull("test/repo-name", &["wiki/First_article/img/test.png"]);

    assert!(compare_pulls(&new_pull, &existing_pull).is_empty());
}

#[test]
fn single_file_overlap() {
    let mut gh = test::GitHubServer::new();

    let existing_pull = gh.make_pull("test/repo-name", &["wiki/Article/en.md"]);
    let new_pull = gh.make_pull("test/repo-name", &["wiki/Article/en.md"]);

    assert_eq!(
        compare_pulls(&new_pull, &existing_pull),
        vec![Conflict::overlap(
            2,
            1,
            gh.url.pull_url("test/repo", 1),
            vec!["wiki/Article/en.md".to_string()],
        )]
    );
}

#[test]
fn multiple_files_overlap() {
    let mut gh = test::GitHubServer::new();

    let existing_pull = gh.make_pull(
        "test/repo-name",
        &[
            "wiki/Article/en.md",
            "wiki/Ranking_criteria/en.md",
            "wiki/Article/img/test.png",
            "wiki/Unrelated_article/ru.md",
        ],
    );
    let new_pull = gh.make_pull(
        "test/repo-name",
        &[
            "wiki/Ranking_criteria/en.md",
            "wiki/Article/en.md",
            "wiki/Some_other_article/en.md",
            "wiki/Test_article/en.md",
        ],
    );

    assert_eq!(
        compare_pulls(&new_pull, &existing_pull),
        vec![Conflict::overlap(
            2,
            1,
            gh.url.pull_url("test/repo", 1),
            vec![
                "wiki/Article/en.md".to_string(),
                "wiki/Ranking_criteria/en.md".to_string(),
            ]
        )]
    );
    assert_eq!(
        compare_pulls(&existing_pull, &new_pull),
        vec![Conflict::overlap(
            1,
            2,
            gh.url.pull_url("test/repo", 2),
            vec![
                "wiki/Article/en.md".to_string(),
                "wiki/Ranking_criteria/en.md".to_string(),
            ]
        )]
    );
}

#[test]
fn existing_translation_becomes_incomplete() {
    let mut gh = test::GitHubServer::new();

    let existing_pull = gh.make_pull("test/repo-name", &["wiki/Article/ru.md"]);
    let new_pull = gh.make_pull("test/repo-name", &["wiki/Article/en.md"]);

    assert_eq!(
        compare_pulls(&new_pull, &existing_pull),
        vec![Conflict::incomplete_translation(
            1,
            2,
            gh.url.pull_url("test/repo", 2),
            vec!["wiki/Article/en.md".to_string(),],
        )]
    );
}

#[test]
fn new_translation_marked_as_incomplete() {
    let mut gh = test::GitHubServer::new();

    let existing_pull = gh.make_pull("test/repo-name", &["wiki/Article/en.md"]);
    let new_pull = gh.make_pull("test/repo-name", &["wiki/Article/ru.md"]);

    assert_eq!(
        compare_pulls(&new_pull, &existing_pull),
        vec![Conflict::incomplete_translation(
            2,
            1,
            gh.url.pull_url("test/repo", 1),
            vec!["wiki/Article/en.md".to_string(),],
        )]
    );
}
