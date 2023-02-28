use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use eyre::Result;

use crate::github;
use crate::structs;

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

pub struct DummyGitHubClient {
    app_id: String,

    // Github mock information
    installations: Arc<Mutex<HashMap<i64, structs::Installation>>>,
    last_pull_id: Arc<Mutex<i64>>,
    pulls: Arc<Mutex<HashMap<String, Vec<structs::PullRequest>>>>,
    last_comment_id: Arc<Mutex<i64>>,
    comments: Arc<Mutex<HashMap<String, HashMap<i32, Vec<structs::IssueComment>>>>>,
}

#[async_trait]
impl github::GitHubInterface for DummyGitHubClient {
    fn new(app_id: String, _key: String) -> Self {
        Self {
            app_id,
            installations: Arc::default(),
            last_pull_id: Arc::new(Mutex::new(1)),
            pulls: Arc::default(),
            last_comment_id: Arc::new(Mutex::new(1)),
            comments: Arc::default(),
        }
    }

    async fn installations(&self) -> Result<Vec<structs::Installation>> {
        Ok(self.cached_installations())
    }

    fn cached_installations(&self) -> Vec<structs::Installation> {
        self.installations
            .lock()
            .unwrap()
            .values()
            .cloned()
            .collect()
    }

    // TODO: set repositories?
    async fn discover_installations(&self) -> Result<Vec<structs::Installation>> {
        Ok(self.cached_installations())
    }

    async fn app(&self) -> Result<structs::App> {
        Ok(structs::App {
            id: self.app_id.parse().unwrap(),
            slug: "test-app".to_string(),
            owner: structs::Actor {
                id: 1,
                login: "test-owner".to_string(),
            },
            name: "Test GitHub app".to_string(),
        })
    }

    // TODO: set repositories?
    async fn add_installation(
        &self,
        installation: structs::Installation,
    ) -> Result<structs::Installation> {
        self.installations
            .lock()
            .unwrap()
            .insert(installation.id, installation.clone());
        Ok(installation)
    }

    fn remove_installation(&self, installation: &structs::Installation) {
        self.installations.lock().unwrap().remove(&installation.id);
    }

    async fn pulls(&self, full_repo_name: &str) -> Result<Vec<structs::PullRequest>> {
        match self.pulls.lock().unwrap().get(&full_repo_name.to_string()) {
            Some(v) => Ok(v.clone()),
            None => Ok(Vec::new()),
        }
    }

    async fn post_comment(
        &self,
        full_repo_name: &str,
        issue_number: i32,
        body: String,
    ) -> Result<()> {
        let now = chrono::Utc::now();
        let mut last_comment_id = self.last_comment_id.lock().unwrap();

        let mut guard = self.comments.lock().unwrap();
        let pull_comments = guard
            .entry(full_repo_name.to_string())
            .or_default()
            .entry(issue_number)
            .or_default();
        pull_comments.push(structs::IssueComment {
            id: *last_comment_id,
            body,
            user: structs::Actor {
                id: 1,
                login: "test-app[bot]".to_string(),
            },
            created_at: now,
            updated_at: now,
        });

        *last_comment_id += 1;
        Ok(())
    }

    async fn update_comment(
        &self,
        full_repo_name: &str,
        comment_id: i64,
        body: String,
    ) -> Result<()> {
        if let Some(comments) = self.comments.lock().unwrap().get_mut(full_repo_name) {
            for pull_comments in comments.values_mut() {
                for c in pull_comments.iter_mut() {
                    if c.id == comment_id {
                        c.body = body;
                        return Ok(());
                    }
                }
            }
        }
        eyre::bail!("no comment {} found", comment_id);
    }

    async fn list_comments(
        &self,
        full_repo_name: &str,
        issue_number: i32,
    ) -> Result<Vec<structs::IssueComment>> {
        if let Some(comments) = self.comments.lock().unwrap().get(full_repo_name) {
            if let Some(pull_comments) = comments.get(&issue_number) {
                return Ok(pull_comments.clone());
            }
        }
        Ok(Vec::new())
    }

    async fn read_pull_diff(
        &self,
        full_repo_name: &str,
        pull_number: i32,
    ) -> Result<unidiff::PatchSet> {
        if let Some(pulls) = self.pulls.lock().unwrap().get(full_repo_name) {
            for p in pulls.iter().filter(|p_| p_.number == pull_number) {
                if let Some(diff) = &p.diff {
                    return Ok(diff.clone());
                }
            }
        }
        eyre::bail!("no diff found for pull {}", pull_number);
    }
}

impl DummyGitHubClient {
    pub fn test_add_pull(&self, full_repo_name: &str, file_names: &[&str]) -> structs::PullRequest {
        let now = chrono::Utc::now();
        let mut last_pull_id = self.last_pull_id.lock().unwrap();
        let pull = structs::PullRequest {
            id: *last_pull_id,
            number: *last_pull_id as i32,
            state: "open".to_string(),
            title: "[FI] Update `Rules`".to_string(),
            user: structs::Actor {
                id: 1,
                login: "LunaticMara".to_string(),
            },
            html_url: format!(
                "https://github.com/{}/pull/{}",
                full_repo_name, *last_pull_id
            ),
            created_at: now,
            updated_at: now,
            diff: Some(make_simple_diff(file_names)),
        };
        *last_pull_id += 1;
        self.pulls
            .lock()
            .unwrap()
            .entry(full_repo_name.to_string())
            .or_default()
            .push(pull.clone());
        pull
    }

    pub fn test_replace_diff(&self, full_repo_name: &str, pull_number: i32, file_names: &[&str]) {
        for p in self
            .pulls
            .lock()
            .unwrap()
            .entry(full_repo_name.to_string())
            .or_default()
        {
            if p.number == pull_number {
                p.diff = Some(make_simple_diff(file_names));
            }
        }
    }
}
