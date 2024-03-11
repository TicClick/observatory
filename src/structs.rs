// https://docs.github.com/developers/webhooks-and-events/webhooks/webhook-events-and-payloads

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// https://docs.github.com/en/rest/users/users
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Actor {
    pub id: i64,
    pub login: String,
}

// https://docs.github.com/en/rest/repos/repos
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Repository {
    pub id: i64,
    pub name: String,
    pub full_name: String,
    pub fork: Option<bool>,   // missing in installation events
    pub owner: Option<Actor>, // missing in installation events
}

// https://docs.github.com/en/rest/pulls/pulls
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PullRequest {
    pub id: i64,
    pub number: i32,
    pub state: String,
    pub title: String,
    pub user: Actor,
    pub html_url: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    #[serde(skip)]
    pub diff: Option<unidiff::PatchSet>,

    #[serde(default)]
    pub merged_at: Option<chrono::DateTime<chrono::Utc>>,

    #[serde(default)]
    pub merged: bool,
}

impl PullRequest {
    pub fn is_merged(&self) -> bool {
        self.merged || self.merged_at.is_some()
    }
}

// https://docs.github.com/en/developers/webhooks-and-events/webhooks/webhook-events-and-payloads#pull_request
#[derive(Debug, Serialize, Deserialize)]
pub struct PullRequestEvent {
    pub action: String,
    pub number: i32,
    pub pull_request: PullRequest,
    pub repository: Repository,
    pub installation: InstallationIdWrapper,
    pub sender: Actor,
}

// https://docs.github.com/webhooks-and-events/webhooks/webhook-events-and-payloads#installation
#[derive(Debug, Serialize, Deserialize)]
pub struct InstallationEvent {
    pub action: String,
    pub installation: Installation,
    pub sender: Actor,

    #[serde(default)]
    pub repositories: Vec<Repository>,
}

// https://docs.github.com/webhooks-and-events/webhooks/webhook-events-and-payloads#installation_repositories
#[derive(Debug, Serialize, Deserialize)]
pub struct InstallationRepositoriesEvent {
    pub action: String,
    pub installation: Installation,
    pub sender: Actor,
    pub repositories_added: Vec<Repository>,
    pub repositories_removed: Vec<Repository>,
}

// Pull request events only contain installation id
#[derive(Debug, Serialize, Deserialize)]
pub struct InstallationIdWrapper {
    pub id: i64,
}

// https://docs.github.com/en/rest/reference/apps#list-installations-for-the-authenticated-app
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Installation {
    pub id: i64,
    pub account: Actor,
    pub app_id: i64,
}

// https://docs.github.com/en/rest/reference/apps#create-an-installation-access-token-for-an-app
#[derive(Debug, Serialize, Deserialize)]
pub struct InstallationToken {
    pub token: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub repositories: Option<Vec<Repository>>,
    pub permissions: HashMap<String, String>,
}

// https://docs.github.com/en/rest/apps/installations#list-repositories-accessible-to-the-app-installation
#[derive(Debug, Serialize, Deserialize)]
pub struct InstallationRepositories {
    pub total_count: i32,
    pub repositories: Vec<Repository>,
}

// https://docs.github.com/en/rest/issues/comments#list-issue-comments
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IssueComment {
    pub id: i64,
    pub body: String,
    pub user: Actor,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

// https://docs.github.com/en/rest/issues/comments#create-an-issue-comment
#[derive(Debug, Serialize, Deserialize)]
pub struct PostIssueComment {
    pub body: String,
}

// https://docs.github.com/en/rest/apps/apps#get-the-authenticated-app
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct App {
    pub id: i64,
    pub slug: String,
    pub owner: Actor,
    pub name: String,
}
