// TODO: document members of the module where it makes sense

use std::str::FromStr;
use std::time::Duration;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use serde::Deserialize;
use serde::Serialize;

use eyre::Result;
use unidiff;

use crate::structs;

const GITHUB_API_ROOT: &str = "https://api.github.com";
const GITHUB_ROOT: &str = "https://github.com";

const RETRYABLE_ERRORS: [u16; 4] = [429, 500, 502, 503];

const MIN_TIMEOUT: Duration = Duration::from_secs(1);
const MAX_TIMEOUT: Duration = Duration::from_secs(30);
const BACKOFF_MP: f32 = 1.2;

/// Helper for exponential backoff retries. Usage:
///
/// ```ignore
/// // Allow up to 3 retries and sleep for 1, 1.2, and 1.44s between them.
/// let mut t = ProgressiveTimeout::new(3);
/// while let None = fetch_data() {
///     t.sleep();
///     if let Err(e) = t.tick() {
///         panic!("failed to fetch data: {e:?}")
///     }
/// }
/// ```
pub struct ProgressiveTimeout {
    current_timeout: Duration,
    current_retry: i32,
    max_retries: i32,
    total_time_slept: Duration,
}

impl ProgressiveTimeout {
    pub fn new(max_retries: i32) -> Self {
        Self {
            current_timeout: MIN_TIMEOUT,
            current_retry: 0,
            max_retries,
            total_time_slept: Duration::new(0, 0),
        }
    }

    pub fn current_timeout(&self) -> Duration {
        self.current_timeout
    }

    pub fn current_retry(&self) -> i32 {
        self.current_retry
    }

    pub fn max_retries(&self) -> i32 {
        self.max_retries
    }

    pub fn tick(&mut self) -> Result<()> {
        if self.current_retry == self.max_retries {
            eyre::bail!(
                "Retries exhausted ({0}/{0}, time slept in total: {1:?})",
                self.max_retries,
                self.total_time_slept
            )
        }
        let new_timeout = std::cmp::min(self.current_timeout.mul_f32(BACKOFF_MP), MAX_TIMEOUT);
        self.current_retry += 1;
        self.current_timeout = new_timeout;
        Ok(())
    }

    pub fn sleep(&mut self) {
        std::thread::sleep(self.current_timeout);
        self.total_time_slept += self.current_timeout;
    }
}

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

    // GitHub.com links

    pub fn pull_url(full_repo_name: &str, pull_number: i32) -> String {
        format!("{GITHUB_ROOT}/{full_repo_name}/pull/{pull_number}")
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

#[async_trait]
pub trait GitHubInterface {
    fn new(app_id: String, key: String) -> Self;
    async fn installations(&self) -> Result<Vec<structs::Installation>>;
    fn cached_installations(&self) -> Vec<structs::Installation>;
    fn update_cached_installation(&self, installation: structs::Installation);
    async fn discover_installations(&self) -> Result<Vec<structs::Installation>>;
    async fn app(&self) -> Result<structs::App>;
    async fn add_installation(
        &self,
        mut installation: structs::Installation,
    ) -> Result<structs::Installation>;
    fn remove_installation(&self, installation: &structs::Installation);
    async fn pulls(&self, full_repo_name: &str) -> Result<Vec<structs::PullRequest>>;
    async fn post_comment(
        &self,
        full_repo_name: &str,
        issue_number: i32,
        body: String,
    ) -> Result<()>;
    async fn update_comment(
        &self,
        full_repo_name: &str,
        comment_id: i64,
        body: String,
    ) -> Result<()>;
    async fn list_comments(
        &self,
        full_repo_name: &str,
        issue_number: i32,
    ) -> Result<Vec<structs::IssueComment>>;
    async fn read_pull_diff(
        &self,
        full_repo_name: &str,
        pull_number: i32,
    ) -> Result<unidiff::PatchSet>;
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

    let mut timer = ProgressiveTimeout::new(10);
    while timer.tick().is_ok() {
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
                    timer.current_retry(),
                    timer.max_retries(),
                );
                if status.is_client_error() || status.is_server_error() || body.is_err() {
                    let can_be_retried = RETRYABLE_ERRORS.contains(&status.as_u16());
                    let log_level = if can_be_retried {
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

                    if can_be_retried {
                        log::info!("Sleeping for {:?}...", timer.current_timeout);
                        timer.sleep();
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
    fn default_headers() -> reqwest::header::HeaderMap {
        let mut m = reqwest::header::HeaderMap::new();
        m.insert("Accept", "application/vnd.github+json".try_into().unwrap());
        m.insert("User-Agent", "observatory".try_into().unwrap());
        m
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

    async fn cached_token(&self, ttype: &TokenType) -> Option<String> {
        let tokens = self.tokens.lock().unwrap();
        if let Some(tt) = tokens.get(ttype) {
            if !tt.expired() {
                return Some(tt.t.clone());
            }
        }
        None
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
}

#[async_trait]
impl GitHubInterface for Client {
    fn new(app_id: String, key: String) -> Self {
        Self {
            app_id,
            key,
            http_client: reqwest::Client::new(),
            tokens: Arc::new(Mutex::new(HashMap::new())),
            installations: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    async fn app(&self) -> Result<structs::App> {
        let pp = self
            .http_client
            .get(GitHub::app())
            .bearer_auth(self.get_jwt_token().await);
        let app: structs::App = __json(pp).await?;
        Ok(app)
    }

    fn cached_installations(&self) -> Vec<structs::Installation> {
        self.installations
            .lock()
            .unwrap()
            .values()
            .cloned()
            .collect()
    }

    fn update_cached_installation(&self, installation: structs::Installation) {
        self.installations
            .lock()
            .unwrap()
            .insert(installation.id, installation);
    }

    // TODO: confirm that this is actually needed (see similar stuff below)
    async fn installations(&self) -> Result<Vec<structs::Installation>> {
        let pp = self
            .http_client
            .get(GitHub::app_installations())
            .bearer_auth(self.get_jwt_token().await);
        let items: Vec<structs::Installation> = __json(pp).await?;
        Ok(items)
    }

    async fn discover_installations(&self) -> Result<Vec<structs::Installation>> {
        let mut ret = Vec::new();
        if let Ok(installations) = self.installations().await {
            for installation in installations {
                ret.push(self.add_installation(installation).await?);
            }
        }
        Ok(ret)
    }

    async fn add_installation(
        &self,
        mut installation: structs::Installation,
    ) -> Result<structs::Installation> {
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
                        self.update_cached_installation(installation.clone());
                        Ok(installation)
                    }
                }
            }
        }
    }

    fn remove_installation(&self, installation: &structs::Installation) {
        self.installations.lock().unwrap().remove(&installation.id);
        self.tokens
            .lock()
            .unwrap()
            .remove(&TokenType::Installation(installation.id));
    }

    async fn pulls(&self, full_repo_name: &str) -> Result<Vec<structs::PullRequest>> {
        let mut out = Vec::new();
        let token = self.pick_token(full_repo_name).await?;
        let per_page = 100;

        for page in 1..100 {
            let req = self
                .http_client
                .get(GitHub::pulls(full_repo_name))
                .query(&[
                    ("state", "open"),
                    ("direction", "asc"),
                    ("sort", "created"),
                    ("per_page", &per_page.to_string()),
                    ("page", &page.to_string()),
                ])
                .bearer_auth(token.clone());
            let mut response: Vec<structs::PullRequest> = __json(req).await?;
            let is_last_page = response.len() < per_page;
            out.append(&mut response);
            if is_last_page {
                break;
            }
        }
        Ok(out)
    }

    async fn post_comment(
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

    async fn update_comment(
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

    async fn list_comments(
        &self,
        full_repo_name: &str,
        issue_number: i32,
    ) -> Result<Vec<structs::IssueComment>> {
        let mut out = Vec::new();
        let token = self.pick_token(full_repo_name).await?;
        let per_page = 100;

        for page in 1..100 {
            let req = self
                .http_client
                .get(GitHub::comments(full_repo_name, issue_number))
                .query(&[
                    ("per_page", &per_page.to_string()),
                    ("page", &page.to_string()),
                ])
                .bearer_auth(token.clone());
            let mut response: Vec<structs::IssueComment> = __json(req).await?;
            let is_last_page = response.len() < per_page;
            out.append(&mut response);
            if is_last_page {
                break;
            }
        }
        Ok(out)
    }

    async fn read_pull_diff(
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
