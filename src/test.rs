use std::collections::HashMap;
use std::str::FromStr;

use crate::github::GitHub;
use crate::structs;

pub static TEST_APP_ID: i64 = 123;

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

pub struct GitHubServer {
    pub server: mockito::ServerGuard,
    pub url: GitHub,

    pub installations: HashMap<i64, structs::Installation>, // installation id -> object
    pub repos: HashMap<i64, HashMap<String, structs::Repository>>, // installation id -> full repository name -> object
    pub pulls: HashMap<String, HashMap<i32, structs::PullRequest>>, // full repository name -> pull number -> object
    pub comments: HashMap<String, HashMap<i32, HashMap<i64, structs::IssueComment>>>, // full repository name -> pull number -> comment id -> object
}

impl GitHubServer {
    pub fn make_app(&self) -> structs::App {
        structs::App {
            id: TEST_APP_ID,
            slug: "test-app".into(),
            owner: structs::Actor {
                id: 1,
                login: "TicClick".into(),
            },
            name: "observatory-test-app".into(),
        }
    }

    pub fn make_installation(&mut self) -> structs::Installation {
        let id = self.installations.len() as i64 + 1;
        let new_installation = structs::Installation {
            id,
            account: structs::Actor {
                id: 1,
                login: "TicClick".into(),
            },
            app_id: TEST_APP_ID,
        };
        self.installations.insert(id, new_installation.clone());
        new_installation
    }

    pub fn make_repo(&mut self, installation_id: i64, full_repo_name: &str) -> structs::Repository {
        let repos = self
            .repos
            .entry(installation_id)
            .or_insert(HashMap::default());
        let id = repos.len() as i64 + 1;

        let new_repo = structs::Repository {
            id,
            name: full_repo_name.split("/").last().unwrap().into(),
            full_name: full_repo_name.into(),
            fork: None,
            owner: None,
        };
        repos.insert(full_repo_name.into(), new_repo.clone());
        new_repo
    }

    pub fn make_pull(&mut self, full_repo_name: &str, file_names: &[&str]) -> structs::PullRequest {
        let pulls = self
            .pulls
            .entry(full_repo_name.into())
            .or_insert(HashMap::default());
        let id = pulls.len() as i64 + 1;
        let number = id as i32;

        let now = chrono::Utc::now();
        let new_pull = structs::PullRequest {
            id,
            number,
            state: "open".to_string(),
            title: "Update `Ranking criteria`".to_string(),
            user: structs::Actor {
                id: 2,
                login: "BanchoBot".to_string(),
            },
            html_url: self.url.pull_url(full_repo_name, number),
            created_at: now,
            updated_at: now,
            diff: Some(make_simple_diff(file_names)),
            merged_at: None,
            merged: false,
        };
        pulls.insert(number, new_pull.clone());
        new_pull
    }

    pub fn change_pull_diff(
        &mut self,
        full_repo_name: &str,
        number: i32,
        file_names: &[&str],
    ) -> structs::PullRequest {
        let pulls = self.pulls.get_mut(full_repo_name).unwrap();
        let pull = pulls.get_mut(&number).unwrap();
        pull.diff = Some(make_simple_diff(file_names));
        pull.updated_at = chrono::Utc::now();
        pull.clone()
    }

    pub fn make_comment(
        &mut self,
        full_repo_name: &str,
        pull_number: i32,
        body: &str,
        author: &str,
    ) -> structs::IssueComment {
        let pulls = self
            .comments
            .entry(full_repo_name.into())
            .or_insert(HashMap::default());
        let comments = pulls.entry(pull_number).or_insert(HashMap::default());

        let id = comments.len() as i64 + 1;

        let created_at = chrono::Utc::now();
        let new_comment = structs::IssueComment {
            id,
            body: body.into(),
            user: structs::Actor {
                id: 1,
                login: author.into(),
            },
            created_at,
            updated_at: created_at,
        };
        comments.insert(id, new_comment.clone());
        new_comment
    }
}

impl GitHubServer {
    pub fn new() -> Self {
        let server = mockito::Server::new();
        let gh = GitHub::new(server.url(), server.url());

        Self {
            server,
            url: gh,
            installations: HashMap::new(),
            repos: HashMap::new(),
            pulls: HashMap::new(),
            comments: HashMap::new(),
        }
    }

    pub fn with_github_app(mut self, app: &structs::App) -> Self {
        self.server
            .mock("GET", "/app")
            .with_status(200)
            .with_body(serde_json::to_string(app).unwrap())
            .create();
        self
    }

    pub fn with_app_installations(
        mut self,
        installations: &[(structs::Installation, Vec<structs::Repository>)],
    ) -> Self {
        let ii: Vec<_> = installations.iter().cloned().map(|i| i.0).collect();

        self.server
            .mock("GET", "/app/installations")
            .with_status(200)
            .with_body(serde_json::to_string(&ii).unwrap())
            .create();

        for (i, rr) in installations {
            let token_str = format!("fake-access-token-installation-{}", i.id);
            let token = structs::InstallationToken {
                token: token_str,
                expires_at: chrono::Utc::now() + chrono::Duration::minutes(30),
                repositories: None,
                permissions: HashMap::<String, String>::from_iter([(
                    "pulls".to_owned(),
                    "write".to_owned(),
                )]),
            };
            self.server
                .mock(
                    "POST",
                    format!("/app/installations/{}/access_tokens", i.id).as_str(),
                )
                .with_status(201)
                .with_body(serde_json::to_string(&token).unwrap())
                .create();

            let repos_response = structs::InstallationRepositories {
                total_count: rr.len() as i32,
                repositories: rr.to_vec(),
            };

            self.server
                .mock("GET", "/installation/repositories")
                .with_status(200)
                .with_body(serde_json::to_string(&repos_response).unwrap())
                .create();

            for r in rr {
                let prs = match self.pulls.get(&r.full_name) {
                    Some(pp) => pp.values().cloned().collect(),
                    None => Vec::new(),
                };
                self.server
                    .mock("GET", format!("/repos/{}/pulls?state=open&direction=asc&sort=created&per_page=100&page=1", r.full_name).as_str())
                    .with_status(200)
                    .with_body(serde_json::to_string(&prs).unwrap())
                    .create();
            }
        }
        self
    }

    pub fn with_pull(mut self, full_repo_name: &str, pull: &structs::PullRequest) -> Self {
        if let Some(ref diff) = pull.diff {
            self.server
                .mock(
                    "GET",
                    format!("/{}/pull/{}.diff", full_repo_name, pull.number).as_str(),
                )
                .with_status(200)
                .with_body(diff.to_string())
                .create();
        }
        self
    }

    pub fn with_pulls(mut self, full_repo_name: &str, pulls: &[structs::PullRequest]) -> Self {
        for p in pulls {
            self = self.with_pull(full_repo_name, p);
        }
        self
    }

    pub fn mock_pull_comments(
        &mut self,
        full_repo_name: &str,
        pull_number: i32,
        expected_body: Option<String>,
    ) -> mockito::Mock {
        let mock = self
            .server
            .mock(
                "POST",
                format!("/repos/{}/issues/{}/comments", full_repo_name, pull_number).as_str(),
            )
            .with_status(200);

        match expected_body {
            None => mock,
            Some(s) => mock.match_body(
                serde_json::to_string(&structs::PostIssueComment { body: s })
                    .unwrap()
                    .as_str(),
            ),
        }
        .create()
    }

    pub fn with_comments(
        mut self,
        full_repo_name: &str,
        pull_number: i32,
        comments: &[structs::IssueComment],
    ) -> Self {
        self.server
            .mock(
                "GET",
                format!(
                    "/repos/{}/issues/{}/comments?per_page=100&page=1",
                    full_repo_name, pull_number
                )
                .as_str(),
            )
            .with_status(200)
            .with_body(serde_json::to_string(&comments).unwrap())
            .create();
        self
    }

    pub fn mock_comment(
        &mut self,
        full_repo_name: &str,
        comment_id: i64,
        expected_body: String,
    ) -> mockito::Mock {
        let mock = self
            .server
            .mock(
                "PATCH",
                format!("/repos/{}/issues/comments/{}", full_repo_name, comment_id).as_str(),
            )
            .match_body(
                serde_json::to_string(&structs::PostIssueComment {
                    body: expected_body,
                })
                .unwrap()
                .as_str(),
            )
            .with_status(200)
            .create();
        mock
    }

    pub fn mock_delete_comment(&mut self, full_repo_name: &str, comment_id: i64) -> mockito::Mock {
        let mock = self
            .server
            .mock(
                "DELETE",
                format!("/repos/{}/issues/comments/{}", full_repo_name, comment_id).as_str(),
            )
            .with_status(200)
            .create();
        mock
    }
}

impl GitHubServer {
    pub fn with_default_github_app(self) -> Self {
        let app = self.make_app();
        self.with_github_app(&app)
    }

    pub fn with_default_app_installations(mut self) -> Self {
        let installation = self.make_installation();
        let repo = self.make_repo(installation.id, "test/repo");
        self.with_app_installations(&[(installation, vec![repo])])
    }
}
