use std::str::FromStr;

use crate::github;
use crate::structs;

pub fn pull_link(full_repo_name: &str, pull_number: i32) -> String {
    github::GitHub::default().pull_url(full_repo_name, pull_number)
}

pub fn make_pull(pull_id: i64, file_names: &[&str]) -> structs::PullRequest {
    let now = chrono::Utc::now();
    structs::PullRequest {
        id: pull_id,
        number: pull_id as i32,
        state: "open".to_string(),
        title: "Update `Ranking criteria`".to_string(),
        user: structs::Actor {
            id: 1,
            login: "BanchoBot".to_string(),
        },
        html_url: pull_link("test/repo", pull_id as i32),
        created_at: now,
        updated_at: now,
        diff: Some(make_simple_diff(file_names)),
    }
}

pub fn make_simple_diff(file_names: &[&str]) -> unidiff::PatchSet {
    let diff: Vec<String> = file_names
        .iter()
        .map(|file_name| {
            format!(
                r#"diff --git a/{0} b/{1}
index 5483f282a0a..2c8c1482b97 100644
--- a/{0}
+++ b/{1}
@@ -5,6 +5,7 @@
 
 ## Test article
 
+<!-- test -->
 Do whatever you want.
 
 That's it, that's the article."#,
                file_name, file_name
            )
        })
        .collect();
    unidiff::PatchSet::from_str(&diff.join("\n")).unwrap()
}
