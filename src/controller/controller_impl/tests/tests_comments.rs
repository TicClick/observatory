use crate::helpers::{conflicts::Conflict, ToMarkdown};

use super::*;

#[allow(unused_assignments)]
#[tokio::test]
async fn test_no_conflict_no_comment() {
    let mut server = GitHubServer::new()
        .await
        .with_default_github_app()
        .with_default_app_installations();

    let c = new_controller(&server, true).await;
    let pull = server.make_pull("test/repo", &["wiki/Article/en.md"]);
    server =
        server
            .with_pull("test/repo", &pull)
            .with_comments("test/repo", pull.number, &Vec::new());

    let c1 = server
        .mock_pull_comments("test/repo", pull.number, None)
        .expect(0);

    c.upsert_pull("test/repo", pull.clone(), true)
        .await
        .unwrap();
    c1.assert();
}

#[tokio::test]
async fn test_one_conflict_one_comment() {
    let mut server = GitHubServer::new()
        .await
        .with_default_github_app()
        .with_default_app_installations();

    let c = new_controller(&server, true).await;
    let p1 = server.make_pull("test/repo", &["wiki/Article/en.md"]);
    let p2 = server.make_pull("test/repo", &["wiki/Article/en.md"]);

    server = server
        .with_pull("test/repo", &p1)
        .with_comments("test/repo", p1.number, &Vec::new())
        .with_pull("test/repo", &p2)
        .with_comments("test/repo", p2.number, &Vec::new());

    let conflict_comment = Conflict::overlap(
        p2.number,
        p1.number,
        p1.html_url.clone(),
        vec!["wiki/Article/en.md".to_string()],
    )
    .to_markdown();

    let c1 = server
        .mock_pull_comments("test/repo", p1.number, None)
        .expect(0);
    let c2 = server
        .mock_pull_comments("test/repo", p2.number, Some(conflict_comment))
        .expect(1);

    c.upsert_pull("test/repo", p1.clone(), true).await.unwrap();
    c.upsert_pull("test/repo", p2.clone(), true).await.unwrap();

    c1.assert();
    c2.assert();
}

#[allow(unused_assignments)]
#[tokio::test]
async fn test_one_pull_and_conflict_one_comment() {
    let mut server = GitHubServer::new()
        .await
        .with_default_github_app()
        .with_default_app_installations();

    let c = new_controller(&server, true).await;
    let pulls = [
        server.make_pull("test/repo", &["wiki/Article/en.md"]),
        server.make_pull(
            "test/repo",
            &["wiki/Article/en.md", "wiki/Other_article/en.md"],
        ),
    ];

    server = server
        .with_pulls("test/repo", &pulls)
        .with_comments("test/repo", pulls[0].number, &Vec::new())
        .with_comments("test/repo", pulls[1].number, &Vec::new());

    let conflict_comment = Conflict::overlap(
        pulls[1].number,
        pulls[0].number,
        pulls[0].html_url.clone(),
        vec!["wiki/Article/en.md".to_string()],
    )
    .to_markdown();
    let c1 = server
        .mock_pull_comments("test/repo", pulls[0].number, None)
        .expect(0);
    let c2 = server
        .mock_pull_comments("test/repo", pulls[1].number, Some(conflict_comment.clone()))
        .expect(1);

    for p in pulls.iter() {
        c.upsert_pull("test/repo", p.clone(), true).await.unwrap();
    }
    c1.assert();
    c2.assert();

    let dummy_comment = server.make_comment(
        "test/repo",
        pulls[1].number,
        conflict_comment.as_str(),
        "test-app[bot]".into(),
    );

    let p1 = server.change_pull_diff(
        "test/repo",
        pulls[0].number,
        &["wiki/Article/en.md", "wiki/Other_article/en.md"],
    );
    server = server.with_pull("test/repo", &p1).with_comments(
        "test/repo",
        pulls[1].number,
        &vec![dummy_comment.clone()],
    );

    let updated_comment_body_both_articles = Conflict::overlap(
        pulls[1].number,
        pulls[0].number,
        pulls[0].html_url.clone(),
        vec![
            "wiki/Article/en.md".to_string(),
            "wiki/Other_article/en.md".to_string(),
        ],
    )
    .to_markdown();

    let mock_comment = server.mock_comment(
        "test/repo",
        dummy_comment.id,
        updated_comment_body_both_articles,
    );

    c.upsert_pull("test/repo", p1, true).await.unwrap();
    mock_comment.assert();

    c1.assert();
    c2.assert();

    let p1 = server.change_pull_diff("test/repo", pulls[0].number, &["wiki/Other_article/en.md"]);
    server = server.with_pull("test/repo", &p1);

    let updated_comment_body_other_article = Conflict::overlap(
        pulls[1].number,
        pulls[0].number,
        pulls[0].html_url.clone(),
        vec!["wiki/Other_article/en.md".to_string()],
    )
    .to_markdown();
    let mock_comment = server.mock_comment(
        "test/repo",
        dummy_comment.id,
        updated_comment_body_other_article,
    );

    c.upsert_pull("test/repo", p1, true).await.unwrap();
    mock_comment.assert();

    c1.assert();
    c2.assert();
}

#[tokio::test]
async fn test_one_pull_and_conflict_one_comment_updated() {
    let mut server = GitHubServer::new()
        .await
        .with_default_github_app()
        .with_default_app_installations();

    let c = new_controller(&server, true).await;
    let pulls = [
        server.make_pull("test/repo", &["wiki/Article/en.md"]),
        server.make_pull(
            "test/repo",
            &["wiki/Article/en.md", "wiki/Other_article/en.md"],
        ),
    ];

    server = server
        .with_pulls("test/repo", &pulls)
        .with_comments("test/repo", pulls[0].number, &Vec::new())
        .with_comments("test/repo", pulls[1].number, &Vec::new());

    let conflict_comment = Conflict::overlap(
        pulls[1].number,
        pulls[0].number,
        pulls[0].html_url.clone(),
        vec!["wiki/Article/en.md".to_string()],
    )
    .to_markdown();

    let c1 = server
        .mock_pull_comments("test/repo", pulls[0].number, None)
        .expect(0);
    let c2 = server
        .mock_pull_comments("test/repo", pulls[1].number, Some(conflict_comment.clone()))
        .expect(1);

    for p in pulls.iter() {
        c.upsert_pull("test/repo", p.clone(), true).await.unwrap();
    }
    c1.assert();
    c2.assert();

    let dummy_comment = server.make_comment(
        "test/repo",
        pulls[1].number,
        conflict_comment.as_str(),
        "test-app[bot]".into(),
    );

    let p1 = server.change_pull_diff(
        "test/repo",
        pulls[0].number,
        &["wiki/Article/en.md", "wiki/Other_article/en.md"],
    );
    server = server.with_pull("test/repo", &p1).with_comments(
        "test/repo",
        pulls[1].number,
        &vec![dummy_comment.clone()],
    );

    let updated_comment_body_both_articles = Conflict::overlap(
        pulls[1].number,
        pulls[0].number,
        pulls[0].html_url.clone(),
        vec![
            "wiki/Article/en.md".to_string(),
            "wiki/Other_article/en.md".to_string(),
        ],
    )
    .to_markdown();
    let mock_comment = server.mock_comment(
        "test/repo",
        dummy_comment.id,
        updated_comment_body_both_articles,
    );

    c.upsert_pull("test/repo", p1, true).await.unwrap();
    mock_comment.assert();

    let p2 = server.change_pull_diff("test/repo", pulls[1].number, &["wiki/Other_article/en.md"]);
    server = server.with_pull("test/repo", &p2).with_comments(
        "test/repo",
        pulls[1].number,
        &vec![dummy_comment.clone()],
    );

    let updated_comment_body_other_article = Conflict::overlap(
        pulls[1].number,
        pulls[0].number,
        pulls[0].html_url.clone(),
        vec!["wiki/Other_article/en.md".to_string()],
    )
    .to_markdown();
    let mock_comment = server.mock_comment(
        "test/repo",
        dummy_comment.id,
        updated_comment_body_other_article,
    );
    c.upsert_pull("test/repo", p2, true).await.unwrap();

    mock_comment.assert();
}

#[tokio::test]
async fn test_post_comment_on_pull_request_merge() {
    let mut server = GitHubServer::new()
        .await
        .with_default_github_app()
        .with_default_app_installations();

    let c = new_controller(&server, true).await;

    let pulls = [
        server.make_pull("test/repo", &["wiki/Article/en.md", "wiki/Article_2/ru.md"]),
        server.make_pull("test/repo", &["wiki/Other_article/en.md"]),
        server.make_pull(
            "test/repo",
            &[
                "wiki/Article/ru.md",       // IncompleteTranslation (1)
                "wiki/Article_2/ru.md",     // Overlap (1)
                "wiki/Other_article/en.md", // Overlap (2)
                "wiki/New_article/ru.md",   // IncompleteTranslation (4)
            ],
        ),
        server.make_pull("test/repo", &["wiki/New_article/en.md"]),
    ];

    server = server
        .with_pulls("test/repo", &pulls)
        .with_comments("test/repo", pulls[0].number, &Vec::new())
        .with_comments("test/repo", pulls[1].number, &Vec::new())
        .with_comments("test/repo", pulls[2].number, &Vec::new())
        .with_comments("test/repo", pulls[3].number, &Vec::new());

    let c1 = server
        .mock_pull_comments("test/repo", pulls[0].number, None)
        .expect(0);
    c.upsert_pull("test/repo", pulls[0].clone(), true)
        .await
        .unwrap();
    c1.assert();

    let c2 = server
        .mock_pull_comments("test/repo", pulls[1].number, None)
        .expect(0);
    c.upsert_pull("test/repo", pulls[1].clone(), true)
        .await
        .unwrap();
    c1.assert();
    c2.assert();

    // Pull #3 triggers 2 comments -- for now.

    let overlap_comment_1 = Conflict::overlap(
        pulls[2].number,
        pulls[0].number,
        pulls[0].html_url.clone(),
        vec!["wiki/Article_2/ru.md".to_string()],
    )
    .to_markdown();
    let c3_overlap_1 = server
        .mock_pull_comments("test/repo", pulls[2].number, Some(overlap_comment_1))
        .expect(1);

    let overlap_comment_2 = Conflict::overlap(
        pulls[2].number,
        pulls[1].number,
        pulls[1].html_url.clone(),
        vec!["wiki/Other_article/en.md".to_string()],
    )
    .to_markdown();
    let c3_overlap_2 = server
        .mock_pull_comments("test/repo", pulls[2].number, Some(overlap_comment_2))
        .expect(1);

    c.upsert_pull("test/repo", pulls[2].clone(), true)
        .await
        .unwrap();
    c1.assert();
    c2.assert();

    c3_overlap_1.assert();
    c3_overlap_2.assert();

    let c4 = server
        .mock_pull_comments("test/repo", pulls[3].number, None)
        .expect(0);
    c.upsert_pull("test/repo", pulls[3].clone(), true)
        .await
        .unwrap();

    c1.assert();
    c2.assert();
    c4.assert();

    // After #4 is added, and #3 is merged, two more comments follow.

    let incomplete_translation_comment_1 = Conflict::incomplete_translation(
        pulls[2].number,
        pulls[0].number,
        pulls[0].html_url.clone(),
        vec!["wiki/Article/en.md".to_string()],
    )
    .to_markdown();
    let c3_incomplete_translation_1 = server
        .mock_pull_comments(
            "test/repo",
            pulls[2].number,
            Some(incomplete_translation_comment_1),
        )
        .expect(1);

    let incomplete_translation_comment_4 = Conflict::incomplete_translation(
        pulls[2].number,
        pulls[3].number,
        pulls[3].html_url.clone(),
        vec!["wiki/New_article/en.md".to_string()],
    )
    .to_markdown();
    let c3_incomplete_translation_4 = server
        .mock_pull_comments(
            "test/repo",
            pulls[2].number,
            Some(incomplete_translation_comment_4),
        )
        .expect(1);

    let mut merged_pull = pulls[2].clone();
    merged_pull.merged = true;
    c.finalize_pull("test/repo", merged_pull).await;

    c3_incomplete_translation_1.assert();
    c3_incomplete_translation_4.assert();
}

#[tokio::test]
async fn test_obsolete_comment_is_removed() {
    let mut server = GitHubServer::new()
        .await
        .with_default_github_app()
        .with_default_app_installations();

    let c = new_controller(&server, true).await;

    let pulls = [
        server.make_pull("test/repo", &["wiki/Article/en.md", "wiki/Article_2/ru.md"]),
        server.make_pull("test/repo", &["wiki/Article/en.md"]),
    ];

    server = server
        .with_pulls("test/repo", &pulls)
        .with_comments("test/repo", pulls[0].number, &Vec::new())
        .with_comments("test/repo", pulls[1].number, &Vec::new());

    let overlap_comment = Conflict::overlap(
        pulls[1].number,
        pulls[0].number,
        pulls[0].html_url.clone(),
        vec!["wiki/Article/en.md".to_string()],
    )
    .to_markdown();

    let first_pull_comments_mock = server
        .mock_pull_comments("test/repo", pulls[0].number, None)
        .expect(0);
    let overlap_mock = server
        .mock_pull_comments("test/repo", pulls[1].number, Some(overlap_comment.clone()))
        .expect(1);

    for p in pulls.iter() {
        c.upsert_pull("test/repo", p.clone(), true).await.unwrap();
    }

    first_pull_comments_mock.assert();
    overlap_mock.assert();

    let p1 = server.change_pull_diff("test/repo", pulls[1].number, &["wiki/Article_2/en.md"]);
    let oc = server.make_comment(
        "test/repo",
        pulls[1].number,
        overlap_comment.as_str(),
        "test-app[bot]".into(),
    );
    server = server.with_pull("test/repo", &p1).with_comments(
        "test/repo",
        pulls[1].number,
        &vec![oc.clone()],
    );

    let delete_comment = server.mock_delete_comment("test/repo", oc.id);
    c.upsert_pull("test/repo", p1.clone(), true).await.unwrap();
    delete_comment.assert();
}

#[tokio::test]
async fn test_only_target_comment_is_removed() {
    let mut server = GitHubServer::new()
        .await
        .with_default_github_app()
        .with_default_app_installations();

    let c = new_controller(&server, true).await;

    let pulls = [
        server.make_pull("test/repo", &["wiki/Article/Other_article/en.md"]),
        server.make_pull("test/repo", &["wiki/Article/Other_article/en.md"]),
        server.make_pull("test/repo", &["wiki/Article/Other_article/en.md"]),
    ];

    server = server
        .with_pulls("test/repo", &pulls)
        .with_comments("test/repo", pulls[0].number, &Vec::new())
        .with_comments("test/repo", pulls[1].number, &Vec::new())
        .with_comments("test/repo", pulls[2].number, &Vec::new());

    let pull1_comments = server
        .mock_pull_comments("test/repo", pulls[0].number, None)
        .expect(0);

    let pull2_overlap1 = Conflict::overlap(
        pulls[1].number,
        pulls[0].number,
        pulls[0].html_url.clone(),
        vec!["wiki/Article/Other_article/en.md".to_string()],
    )
    .to_markdown();
    let pull2_overlap1_comment = server.make_comment(
        "test/repo",
        pulls[1].number,
        pull2_overlap1.as_str(),
        "test-app[bot]".into(),
    );
    let pull2_overlap1_mock = server
        .mock_pull_comments("test/repo", pulls[1].number, Some(pull2_overlap1.clone()))
        .expect(1);

    let pull3_overlap1 = Conflict::overlap(
        pulls[2].number,
        pulls[0].number,
        pulls[0].html_url.clone(),
        vec!["wiki/Article/Other_article/en.md".to_string()],
    )
    .to_markdown();
    let pull3_overlap1_comment = server.make_comment(
        "test/repo",
        pulls[2].number,
        pull3_overlap1.as_str(),
        "test-app[bot]".into(),
    );
    let pull3_overlap1_mock = server
        .mock_pull_comments("test/repo", pulls[2].number, Some(pull3_overlap1.clone()))
        .expect(1);

    let pull3_overlap2 = Conflict::overlap(
        pulls[2].number,
        pulls[1].number,
        pulls[1].html_url.clone(),
        vec!["wiki/Article/Other_article/en.md".to_string()],
    )
    .to_markdown();
    let pull3_overlap2_comment = server.make_comment(
        "test/repo",
        pulls[2].number,
        pull3_overlap2.as_str(),
        "test-app[bot]".into(),
    );
    let pull3_overlap2_mock = server
        .mock_pull_comments("test/repo", pulls[2].number, Some(pull3_overlap2.clone()))
        .expect(1);

    server = server
        .with_comments(
            "test/repo",
            pulls[1].number,
            &vec![pull2_overlap1_comment.clone()],
        )
        .with_comments(
            "test/repo",
            pulls[2].number,
            &vec![
                pull3_overlap1_comment.clone(),
                pull3_overlap2_comment.clone(),
            ],
        );

    for p in pulls.iter() {
        c.upsert_pull("test/repo", p.clone(), true).await.unwrap();
    }

    pull1_comments.assert();

    pull2_overlap1_mock.assert();

    pull3_overlap1_mock.assert();
    pull3_overlap2_mock.assert();

    let p2 = server.change_pull_diff("test/repo", pulls[1].number, &["wiki/No/en.md"]);

    server = server.with_pull("test/repo", &p2);

    let pull2_delete_overlap1_comment = server
        .mock_delete_comment("test/repo", pull2_overlap1_comment.id)
        .expect(1);

    let pull3_delete_overlap2_comment = server
        .mock_delete_comment("test/repo", pull3_overlap2_comment.id)
        .expect(1);

    let pull3_delete_overlap1_comment = server
        .mock_delete_comment("test/repo", pull3_overlap1_comment.id)
        .expect(0);

    c.upsert_pull("test/repo", p2.clone(), true).await.unwrap();

    pull2_delete_overlap1_comment.assert();
    pull3_delete_overlap1_comment.assert();
    pull3_delete_overlap2_comment.assert();
}

#[allow(unused_assignments)]
#[tokio::test]
async fn test_new_comment_is_posted_after_removal_in_different_pull() {
    let mut server = GitHubServer::new()
        .await
        .with_default_github_app()
        .with_default_app_installations();

    let c = new_controller(&server, true).await;

    let pulls = [
        server.make_pull("test/repo", &["wiki/Article/ru.md"]),
        server.make_pull("test/repo", &["wiki/Article/ru.md"]),
    ];

    server = server
        .with_pulls("test/repo", &pulls)
        .with_comments("test/repo", pulls[0].number, &Vec::new())
        .with_comments("test/repo", pulls[1].number, &Vec::new());

    let overlap_text = Conflict::overlap(
        pulls[1].number,
        pulls[0].number,
        pulls[0].html_url.clone(),
        vec!["wiki/Article/ru.md".to_string()],
    )
    .to_markdown();
    let overlap_comment = server.make_comment(
        "test/repo",
        pulls[1].number,
        overlap_text.as_str(),
        "test-app[bot]".into(),
    );
    let overlap_mock = server
        .mock_pull_comments("test/repo", pulls[1].number, Some(overlap_text.clone()))
        .expect(1);

    for p in &pulls {
        c.upsert_pull("test/repo", p.clone(), true).await.unwrap();
    }
    overlap_mock.assert();

    let p1 = server.change_pull_diff(
        "test/repo",
        pulls[0].number,
        &["wiki/Article/Other_article/en.md"],
    );

    server = server.with_pull("test/repo", &p1).with_comments(
        "test/repo",
        pulls[1].number,
        &vec![overlap_comment.clone()],
    );

    let delete_overlap_comment = server
        .mock_delete_comment("test/repo", overlap_comment.id)
        .expect(1);
    c.upsert_pull("test/repo", p1, true).await.unwrap();
    delete_overlap_comment.assert();

    let updated_p1 = server.change_pull_diff("test/repo", pulls[0].number, &["wiki/Article/ru.md"]);

    let new_overlap_text = Conflict::overlap(
        pulls[0].number,
        pulls[1].number,
        pulls[1].html_url.clone(),
        vec!["wiki/Article/ru.md".to_string()],
    )
    .to_markdown();
    let new_overlap_mock = server
        .mock_pull_comments("test/repo", pulls[0].number, Some(new_overlap_text.clone()))
        .expect(1);

    server = server.with_pull("test/repo", &updated_p1);

    c.upsert_pull("test/repo", updated_p1, true).await.unwrap();
    new_overlap_mock.assert();
}
