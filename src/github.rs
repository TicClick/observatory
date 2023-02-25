// TODO: document members of the module where it makes sense

use std::str::FromStr;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde::Deserialize;
use serde::Serialize;

use eyre::Result;
use unidiff;

use crate::structs;

const GITHUB_API_ROOT: &str = "https://api.github.com";
const GITHUB_ROOT: &str = "https://github.com";

const RETRIES: i32 = 3;
const RETRYABLE_ERRORS: [u16; 4] = [429, 500, 502, 503];

pub struct GitHub {}
impl GitHub {
    pub fn pulls(full_repo_name: &str) -> String {
        format!("{GITHUB_API_ROOT}/repos/{full_repo_name}/pulls")
    }
    pub fn app() -> String {
        format!("{GITHUB_API_ROOT}/app")
    }
    pub fn app_installations() -> String {
        format!("{GITHUB_API_ROOT}/app/installations")
    }
    pub fn installation_tokens(installation_id: i64) -> String {
        format!("{GITHUB_API_ROOT}/app/installations/{installation_id}/access_tokens")
    }
    pub fn installation_repos() -> String {
        format!("{GITHUB_API_ROOT}/installation/repositories")
    }
    pub fn comments(full_repo_name: &str, issue_number: i32) -> String {
        format!("{GITHUB_API_ROOT}/repos/{full_repo_name}/issues/{issue_number}/comments")
    }
    pub fn issue_comment(full_repo_name: &str, comment_id: i64) -> String {
        format!("{GITHUB_API_ROOT}/repos/{full_repo_name}/issues/comments/{comment_id}")
    }
    pub fn diff_url(full_repo_name: &str, pull_number: i32) -> String {
        // Diff links are handled by github.com, not the API subdomain.
        format!("{GITHUB_ROOT}/{full_repo_name}/pull/{pull_number}.diff")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TokenType {
    JWT,
    Installation(i64),
}

#[derive(Debug, Clone)]
pub struct Token {
    pub t: String,
    pub ttype: TokenType,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

impl Token {
    pub fn expired(&self) -> bool {
        chrono::Utc::now() >= self.expires_at
    }
}

#[derive(Debug, Clone)]
pub struct Client {
    app_id: String,
    key: String,
    http_client: reqwest::Client,

    tokens: Arc<Mutex<HashMap<TokenType, Token>>>,
    pub installations: Arc<Mutex<HashMap<i64, structs::Installation>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    exp: usize,
    iat: usize,
    iss: String,

    #[serde(skip)]
    pub created_at: chrono::DateTime<chrono::Utc>,
    #[serde(skip)]
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

impl Claims {
    pub fn new(app_id: &str) -> Self {
        let now = chrono::Utc::now();
        let created_at = now - chrono::Duration::minutes(1);
        let expires_at = now + chrono::Duration::minutes(7);
        Self {
            iat: created_at.timestamp().try_into().unwrap(),
            exp: expires_at.timestamp().try_into().unwrap(),
            iss: app_id.to_owned(),
            created_at,
            expires_at,
        }
    }
}

async fn __json<T>(rb: reqwest::RequestBuilder) -> Result<T>
where
    T: for<'de> serde::Deserialize<'de>,
{
    __text(rb)
        .await
        .map(|body| Ok(serde_json::from_str(&body)?))?
}

const INTERESTING_HEADERS: [&str; 7] = [
    "etag",
    "x-ratelimit-limit",
    "x-ratelimit-remaining",
    "x-ratelimit-reset",
    "x-ratelimit-used",
    "x-ratelimit-resource",
    "x-github-request-id",
];

async fn __text(rb: reqwest::RequestBuilder) -> Result<String> {
    let prepared_request = rb.headers(Client::default_headers());
    let mut url: Option<reqwest::Url> = None;
    for attempt in 0..RETRIES {
        match prepared_request.try_clone().unwrap().send().await {
            Ok(response) => {
                // Yes, you have to deconstruct the response by itself if you step from the trodden path
                // (access URL and body, and do status checks at the same time).
                // https://github.com/seanmonstar/reqwest/issues/1542
                let headers: HashMap<String, String> = response
                    .headers()
                    .iter()
                    .map(|(k, v)| {
                        (
                            k.to_string().to_lowercase(),
                            v.to_str().unwrap_or("<none>").to_owned(),
                        )
                    })
                    .filter(|(k, _)| INTERESTING_HEADERS.contains(&k.as_str()))
                    .collect();
                let status = response.status();
                url = Some(response.url().clone());
                let body = response.text().await;

                let logging_string = format!(
                    "HTTP {} {} ({}/{})",
                    status,
                    url.as_ref().unwrap(),
                    attempt + 1,
                    RETRIES,
                );
                if status.is_client_error() || status.is_server_error() || body.is_err() {
                    let can_be_retried = RETRYABLE_ERRORS.contains(&status.as_u16());
                    let log_level = if can_be_retried && attempt < RETRIES - 1 {
                        log::Level::Warn
                    } else {
                        log::Level::Error
                    };
                    log::log!(
                        log_level,
                        "{}. Headers: {:?} + body: {:?}",
                        logging_string,
                        headers,
                        body
                    );

                    // This will correctly end the retry loop if attempt == RETRIES - 1
                    if can_be_retried {
                        continue;
                    }
                    eyre::bail!(logging_string);
                }

                log::debug!("{}. Headers: {:?}", logging_string, headers);
                return Ok(body.unwrap());
            }
            Err(e) => {
                log::error!(
                    "Error at {}: HTTP {:?}: {:?}",
                    e.url().unwrap(),
                    e.status(),
                    e
                );
                return Err(e.into());
            }
        }
    }
    eyre::bail!("Exhausted retries for {:?}, giving up", url)
}

impl Client {
    pub fn new(app_id: String, key: String) -> Self {
        Self {
            app_id,
            key,
            http_client: reqwest::Client::new(),
            tokens: Arc::new(Mutex::new(HashMap::new())),
            installations: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    async fn cached_token(&self, ttype: &TokenType) -> Option<String> {
        let tokens = self.tokens.lock().unwrap();
        if let Some(tt) = tokens.get(ttype) {
            if !tt.expired() {
                return Some(tt.t.clone());
            }
        }
        None
    }

    // https://docs.github.com/en/developers/apps/building-github-apps/authenticating-with-github-apps#generating-a-json-web-token-jwt
    fn generate_jwt(&self) -> Token {
        let claims = Claims::new(&self.app_id);
        let t = jsonwebtoken::encode(
            &jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256),
            &claims,
            &jsonwebtoken::EncodingKey::from_rsa_pem(self.key.as_bytes()).unwrap(),
        )
        .expect("failed to generate JWT");
        Token {
            t,
            ttype: TokenType::JWT,
            created_at: claims.created_at,
            expires_at: claims.expires_at,
        }
    }

    async fn get_jwt_token(&self) -> String {
        let ttype = TokenType::JWT;
        match self.cached_token(&ttype).await {
            Some(t) => t,
            None => {
                let token = self.generate_jwt();
                self.tokens.lock().unwrap().insert(ttype, token.clone());
                token.t
            }
        }
    }

    async fn get_installation_token(&self, installation_id: i64) -> Result<String> {
        let ttype = TokenType::Installation(installation_id);
        match self.cached_token(&ttype).await {
            Some(t) => Ok(t),
            None => {
                let jwt = self.get_jwt_token().await;
                let req = self
                    .http_client
                    .post(GitHub::installation_tokens(installation_id))
                    .bearer_auth(jwt);
                let response: structs::InstallationToken = __json(req).await?;
                let token = Token {
                    t: response.token,
                    ttype: ttype.clone(),
                    created_at: chrono::Utc::now(),
                    expires_at: response.expires_at - chrono::Duration::minutes(5),
                };
                self.tokens.lock().unwrap().insert(ttype, token.clone());
                Ok(token.t)
            }
        }
    }

    fn default_headers() -> reqwest::header::HeaderMap {
        let mut m = reqwest::header::HeaderMap::new();
        m.insert("Accept", "application/vnd.github+json".try_into().unwrap());
        m.insert("User-Agent", "observatory".try_into().unwrap());
        m
    }

    pub async fn installations(&self) -> Result<Vec<structs::Installation>> {
        let pp = self
            .http_client
            .get(GitHub::app_installations())
            .bearer_auth(self.get_jwt_token().await);
        let items: Vec<structs::Installation> = __json(pp).await?;
        Ok(items)
    }

    pub async fn discover_installations(&self) -> Result<()> {
        if let Ok(installations) = self.installations().await {
            for installation in installations {
                self.add_installation(installation).await?;
            }
        }
        Ok(())
    }

    pub async fn app(&self) -> Result<structs::App> {
        let pp = self
            .http_client
            .get(GitHub::app())
            .bearer_auth(self.get_jwt_token().await);
        let app: structs::App = __json(pp).await?;
        Ok(app)
    }

    pub async fn add_installation(&self, mut installation: structs::Installation) -> Result<()> {
        match self.get_installation_token(installation.id).await {
            Err(e) => {
                log::error!(
                    "Failed to get token for installation {}: {:?}",
                    installation.id,
                    e
                );
                Err(e)
            }
            Ok(token) => {
                let req = self
                    .http_client
                    .get(GitHub::installation_repos())
                    .bearer_auth(token);
                match __json::<structs::InstallationRepositories>(req).await {
                    Err(e) => {
                        log::error!("Failed to fetch list of repositories for a fresh installation {}: {:?}", installation.id, e);
                        Err(e)
                    }
                    Ok(response) => {
                        installation.repositories = response.repositories;
                        self.installations
                            .lock()
                            .unwrap()
                            .insert(installation.id, installation);
                        Ok(())
                    }
                }
            }
        }
    }

    pub fn remove_installation(&self, installation: &structs::Installation) {
        self.installations.lock().unwrap().remove(&installation.id);
        self.tokens
            .lock()
            .unwrap()
            .remove(&TokenType::Installation(installation.id));
    }

    async fn pick_token(&self, full_repo_name: &str) -> Result<String> {
        let mut installation_id = None;
        for (k, v) in self.installations.lock().unwrap().iter() {
            if v.repositories.iter().any(|r| r.full_name == full_repo_name) {
                installation_id = Some(*k);
                break;
            }
        }
        match installation_id {
            None => eyre::bail!("No GitHub token for {} found", full_repo_name),
            Some(iid) => self.get_installation_token(iid).await,
        }
    }

    pub async fn pulls(&self, full_repo_name: &str) -> Result<Vec<structs::PullRequest>> {
        let mut out = Vec::new();
        let token = self.pick_token(full_repo_name).await?;
        for page in 1..100 {
            let req = self
                .http_client
                .get(GitHub::pulls(full_repo_name))
                .query(&[
                    ("state", "open"),
                    ("direction", "asc"),
                    ("sort", "created"),
                    ("per_page", "100"),
                    ("page", &page.to_string()),
                ])
                .bearer_auth(token.clone());
            let mut response: Vec<structs::PullRequest> = __json(req).await?;
            if response.is_empty() {
                break;
            }
            out.append(&mut response);
        }
        Ok(out)
    }

    pub async fn post_comment(
        &self,
        full_repo_name: &str,
        issue_number: i32,
        body: String,
    ) -> Result<()> {
        let comment = serde_json::to_string(&structs::PostIssueComment { body }).unwrap();
        let token = self.pick_token(full_repo_name).await?;
        let req = self
            .http_client
            .post(GitHub::comments(full_repo_name, issue_number))
            .body(comment)
            .bearer_auth(token);
        __json::<structs::IssueComment>(req).await?;
        Ok(())
    }

    pub async fn update_comment(
        &self,
        full_repo_name: &str,
        comment_id: i64,
        body: String,
    ) -> Result<()> {
        let comment = serde_json::to_string(&structs::PostIssueComment { body }).unwrap();
        let token = self.pick_token(full_repo_name).await?;
        let req = self
            .http_client
            .patch(GitHub::issue_comment(full_repo_name, comment_id))
            .body(comment)
            .bearer_auth(token);
        __json::<structs::IssueComment>(req).await?;
        Ok(())
    }

    pub async fn list_comments(
        &self,
        full_repo_name: &str,
        issue_number: i32,
    ) -> Result<Vec<structs::IssueComment>> {
        let mut out = Vec::new();
        let token = self.pick_token(full_repo_name).await?;
        for page in 1..100 {
            let req = self
                .http_client
                .get(GitHub::comments(full_repo_name, issue_number))
                .query(&[("per_page", "100"), ("page", &page.to_string())])
                .bearer_auth(token.clone());
            let mut response: Vec<structs::IssueComment> = __json(req).await?;
            if response.is_empty() {
                break;
            }
            out.append(&mut response);
        }
        Ok(out)
    }

    pub async fn read_pull_diff(
        &self,
        full_repo_name: &str,
        pull_number: i32,
    ) -> Result<unidiff::PatchSet> {
        let token = self.pick_token(full_repo_name).await?;
        let req = self
            .http_client
            .get(GitHub::diff_url(full_repo_name, pull_number))
            .bearer_auth(token);
        let response = __text(req).await?;
        Ok(unidiff::PatchSet::from_str(&response)?)
    }
}

// TODO: add tests
