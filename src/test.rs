use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use eyre::Result;

use crate::github;
use crate::structs;

pub fn pull_link(full_repo_name: &str, pull_number: i32) -> String {
    github::GitHub::pull_url(full_repo_name, pull_number)
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

pub struct DummyGitHubClient {
    app_id: String,

    // Github mock information
    last_installation_id: Arc<Mutex<i64>>,
    installations: Arc<Mutex<HashMap<i64, structs::Installation>>>,

    last_pull_id: Arc<Mutex<i64>>,
    pulls: Arc<Mutex<HashMap<String, Vec<structs::PullRequest>>>>,

    last_comment_id: Arc<Mutex<i64>>,
    comments: Arc<Mutex<HashMap<String, HashMap<i32, Vec<structs::IssueComment>>>>>,

    last_repo_id: Arc<Mutex<i64>>,
    repositories: Arc<Mutex<HashMap<i64, Vec<structs::Repository>>>>,
}

#[async_trait]
impl github::GitHubInterface for DummyGitHubClient {
    fn new(app_id: String, _key: String) -> Self {
        Self {
            app_id,

            last_installation_id: Arc::new(Mutex::new(1)),
            installations: Arc::default(),

            last_pull_id: Arc::new(Mutex::new(1)),
            pulls: Arc::default(),

            last_comment_id: Arc::new(Mutex::new(1)),
            comments: Arc::default(),

            last_repo_id: Arc::new(Mutex::new(1)),
            repositories: Arc::default(),
        }
    }

    async fn installations(&self) -> Result<Vec<structs::Installation>> {
        Ok(self.cached_installations())
    }

    fn cached_installations(&self) -> Vec<structs::Installation> {
        let mut ii: Vec<_> = self
            .installations
            .lock()
            .unwrap()
            .values()
            .cloned()
            .collect();
        for i in ii.iter_mut() {
            i.repositories = self
                .repositories
                .lock()
                .unwrap()
                .entry(i.id)
                .or_default()
                .to_vec();
        }
        ii
    }

    fn add_repositories(&self, installation_id: i64, mut repositories: Vec<structs::Repository>) {
        if let Some(repos) = self.repositories.lock().unwrap().get_mut(&installation_id) {
            let ids: Vec<_> = repositories.iter().map(|r| r.id).collect();
            repos.retain(|r| !ids.contains(&r.id));
            repos.append(&mut repositories);
        }
    }

    fn remove_repositories(&self, installation_id: i64, repositories: &[structs::Repository]) {
        if let Some(repos) = self.repositories.lock().unwrap().get_mut(&installation_id) {
            let ids: Vec<_> = repositories.iter().map(|r| r.id).collect();
            repos.retain(|r| !ids.contains(&r.id));
        }
    }

    async fn discover_installations(&self) -> Result<Vec<structs::Installation>> {
        let mut cached = self.cached_installations();
        for i in cached.iter_mut() {
            i.repositories = self
                .repositories
                .lock()
                .unwrap()
                .entry(i.id)
                .or_default()
                .to_vec();
        }
        Ok(cached)
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
        mut installation: structs::Installation,
    ) -> Result<structs::Installation> {
        self.installations
            .lock()
            .unwrap()
            .insert(installation.id, installation.clone());
        installation.repositories = self
            .repositories
            .lock()
            .unwrap()
            .entry(installation.id)
            .or_default()
            .to_vec();
        Ok(installation)
    }

    fn remove_installation(&self, installation: &structs::Installation) {
        self.installations.lock().unwrap().remove(&installation.id);
        self.repositories.lock().unwrap().remove(&installation.id);
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
            for comments_per_pull in comments.values_mut() {
                for c in comments_per_pull.iter_mut() {
                    if c.id == comment_id {
                        c.body = body;
                        c.updated_at = chrono::Utc::now();
                        return Ok(());
                    }
                }
            }
        }
        eyre::bail!("no comment {} found", comment_id);
    }

    async fn delete_comment(&self, full_repo_name: &str, comment_id: i64) -> Result<()> {
        let mut found = false;
        if let Some(comments) = self.comments.lock().unwrap().get_mut(full_repo_name) {
            for comments_per_pull in comments.values_mut() {
                comments_per_pull.retain(|c| {
                    if c.id == comment_id {
                        found = true;
                    }
                    c.id != comment_id
                });
            }
        }
        if !found {
            eyre::bail!("no comment {} found", comment_id);
        }
        Ok(())
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
    pub fn test_add_installation(&self) -> structs::Installation {
        let mut last_installation_id = self.last_installation_id.lock().unwrap();
        let inst = structs::Installation {
            id: *last_installation_id,
            account: structs::Actor {
                id: 12,
                login: "test-user".into(),
            },
            app_id: 123,
            repositories: Vec::new(),
        };
        self.installations
            .lock()
            .unwrap()
            .insert(*last_installation_id, inst.clone());
        *last_installation_id += 1;
        inst
    }

    pub fn test_add_repository(
        &self,
        installation_id: i64,
        full_repo_name: &str,
    ) -> structs::Repository {
        let mut last_repo_id = self.last_repo_id.lock().unwrap();
        let r = structs::Repository {
            id: *last_repo_id,
            name: full_repo_name.split("/").last().unwrap().into(),
            full_name: full_repo_name.into(),
            fork: None,
            owner: None,
        };

        *last_repo_id += 1;
        self.repositories
            .lock()
            .unwrap()
            .entry(installation_id)
            .or_default()
            .push(r.clone());
        r
    }

    pub fn test_add_pull(&self, full_repo_name: &str, file_names: &[&str]) -> structs::PullRequest {
        let mut last_pull_id = self.last_pull_id.lock().unwrap();
        let pull = make_pull(*last_pull_id, file_names);
        *last_pull_id += 1;
        self.pulls
            .lock()
            .unwrap()
            .entry(full_repo_name.to_string())
            .or_default()
            .push(pull.clone());
        pull
    }

    pub fn test_update_pull(&self, full_repo_name: &str, pull_number: i32, file_names: &[&str]) {
        for p in self
            .pulls
            .lock()
            .unwrap()
            .entry(full_repo_name.to_string())
            .or_default()
        {
            if p.number == pull_number {
                p.diff = Some(make_simple_diff(file_names));
                p.updated_at = chrono::Utc::now();
                return;
            }
        }
        panic!("no pull #{pull_number}");
    }

    pub fn fetch_pull(&self, full_repo_name: &str, pull_number: i32) -> structs::PullRequest {
        for p in self
            .pulls
            .lock()
            .unwrap()
            .entry(full_repo_name.to_string())
            .or_default()
        {
            if p.number == pull_number {
                return p.clone();
            }
        }
        panic!("no pull #{pull_number}");
    }
}
