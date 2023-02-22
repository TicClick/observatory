// TODO: document members of the module where it makes sense

use std::cmp::{PartialEq, PartialOrd};
use std::collections::HashMap;

use eyre::Result;

use crate::{github, memory, structs};

const EXISTING_CHANGE_TEMPLATE: &str = "## Possible conflicts\n
Someone else has edited same files as you did. Please check their changes in case they conflict with yours:\n";

const NEW_ORIGINAL_CHANGE_TEMPLATE: &str = "## New changes\n
There are new changes in the articles you have translated. Please update your translation after they are merged:\n";

const EXISTING_ORIGINAL_CHANGE_TEMPLATE: &str = "## Existing changes\n
There are existing unapplied changes in the articles you have translated. Please update your translation after they are merged:\n";

#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub enum UpdateKind {
    /// Two pull requests have common file(s).
    ExistingChange,
    /// A new pull request affects an article for which there's a translation open.
    NewOriginalChange,
    /// There is a new translation of the article that has a pending change.
    ExistingOriginalChange,
}

fn make_hints() -> HashMap<UpdateKind, String> {
    let mut out = HashMap::new();
    out.insert(
        UpdateKind::ExistingChange,
        String::from(EXISTING_CHANGE_TEMPLATE),
    );
    out.insert(
        UpdateKind::NewOriginalChange,
        String::from(NEW_ORIGINAL_CHANGE_TEMPLATE),
    );
    out.insert(
        UpdateKind::ExistingOriginalChange,
        String::from(EXISTING_ORIGINAL_CHANGE_TEMPLATE),
    );
    out
}

#[derive(Debug, Ord, Eq, PartialEq, PartialOrd)]
pub struct Update {
    pub kind: UpdateKind,
    pub notification_target: i32,
    pub reference_target: i32,
    pub reference_url: String,
    pub file_set: Vec<String>,
}

pub struct Article {
    pub path: String,
    pub language: String,
}

impl Article {
    pub fn from_file_path(s: &str) -> Self {
        let fp = std::path::Path::new(s);
        let language = fp.file_stem().unwrap().to_str().unwrap().to_owned();
        let path = fp.parent().unwrap().to_str().unwrap().to_owned();
        Self { path, language }
    }

    pub fn file_path(&self) -> String {
        format!("{}/{}.md", self.path, self.language)
    }

    pub fn is_original(&self) -> bool {
        self.language == "en"
    }

    pub fn is_translation(&self) -> bool {
        !self.is_original()
    }
}

impl std::cmp::PartialEq for Article {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path && self.language == other.language
    }
}

fn compare(new_pull: &structs::PullRequest, other_pull: &structs::PullRequest) -> Vec<Update> {
    let new_diff = new_pull.diff.as_ref().unwrap();
    let other_diff = other_pull.diff.as_ref().unwrap();

    let mut overlaps = Vec::new();
    let mut originals = Vec::new();
    let mut translations = Vec::new();

    for incoming in new_diff
        .files()
        .iter()
        .filter(|fp| fp.target_file.ends_with(".md"))
    {
        for other in other_diff
            .files()
            .iter()
            .filter(|fp| fp.target_file.ends_with(".md"))
        {
            let new_article = Article::from_file_path(&incoming.path());
            let other_article = Article::from_file_path(&other.path());

            // Different folders.
            if new_article.path != other_article.path {
                continue;
            }

            if new_article == other_article {
                overlaps.push(new_article.file_path());
                continue;
            }

            if new_article.is_original() && other_article.is_translation() {
                originals.push(new_article.file_path());
            } else if new_article.is_translation() && other_article.is_original() {
                translations.push(new_article.file_path());
            }
        }
    }

    let mut out = Vec::new();
    if !overlaps.is_empty() {
        out.push(Update {
            kind: UpdateKind::ExistingChange,
            notification_target: new_pull.number,
            reference_target: other_pull.number,
            reference_url: other_pull.html_url.clone(),
            file_set: overlaps,
        });
    }
    if !originals.is_empty() {
        out.push(Update {
            kind: UpdateKind::NewOriginalChange,
            notification_target: other_pull.number,
            reference_target: new_pull.number,
            reference_url: new_pull.html_url.clone(),
            file_set: originals,
        })
    }
    if !translations.is_empty() {
        out.push(Update {
            kind: UpdateKind::ExistingOriginalChange,
            notification_target: new_pull.number,
            reference_target: other_pull.number,
            reference_url: other_pull.html_url.clone(),
            file_set: translations,
        })
    }
    out
}

#[derive(Debug, Clone)]
pub struct Controller {
    github: github::Client,
    memory: memory::Memory,
}

impl Controller {
    pub fn new(app_id: String, private_key: String) -> Self {
        Self {
            github: github::Client::new(app_id, private_key),
            memory: memory::Memory::new(),
        }
    }

    pub fn installations(&self) -> Vec<structs::Installation> {
        self.github
            .installations
            .lock()
            .unwrap()
            .values()
            .cloned()
            .collect()
    }

    pub async fn init(&self) -> Result<()> {
        self.github.discover_installations().await?;
        for i in self.installations() {
            for r in i.repositories {
                for p in self.github.pulls(&r.full_name).await? {
                    self.add_pull(&r.full_name, p, false).await?;
                }
            }
        }
        Ok(())
    }

    pub async fn add_installation(&self, installation: structs::Installation) -> Result<()> {
        self.github.add_installation(installation.clone()).await?;
        for r in installation.repositories {
            for p in self.github.pulls(&r.full_name).await? {
                self.add_pull(&r.full_name, p, false).await?;
            }
        }
        Ok(())
    }

    pub fn remove_installation(&self, installation: structs::Installation) {
        self.github.remove_installation(&installation);
        for r in installation.repositories {
            self.memory.drop_repository(&r.full_name);
        }
    }

    pub async fn remove_pull(&self, full_repo_name: &str, closed_pull: structs::PullRequest) {
        self.memory.remove(full_repo_name, &closed_pull);
    }

    // TODO: add and remove pulls based on a repository which they are sent against
    pub async fn add_pull(
        &self,
        full_repo_name: &str,
        mut new_pull: structs::PullRequest,
        trigger_updates: bool,
    ) -> Result<()> {
        let diff = self
            .github
            .read_pull_diff(full_repo_name, new_pull.number)
            .await?;
        new_pull.diff = Some(diff);
        self.memory.insert(full_repo_name, new_pull.clone());
        if !trigger_updates {
            return Ok(());
        }

        let mut pending_updates: HashMap<i32, Vec<Update>> = HashMap::new();
        if let Some(pulls_map) = self.memory.pulls.lock().unwrap().get(full_repo_name) {
            for other_pull in pulls_map
                .values()
                .filter(|other| other.number != new_pull.number)
            {
                let updates = compare(&new_pull, other_pull);
                for update in updates {
                    pending_updates
                        .entry(update.notification_target)
                        .or_default()
                        .push(update);
                }
            }
        }
        self.send_updates(pending_updates, full_repo_name).await?;
        Ok(())
    }

    // TODO: this function posts a new comment every time anything changes in pull requests, which is terrible.
    // Instead, it should update its existing comment, and maybe add an empty checkbox to the PR owner's post, and reset it on every update.
    pub async fn send_updates(
        &self,
        pending: HashMap<i32, Vec<Update>>,
        full_repo_name: &str,
    ) -> Result<()> {
        for (target, mut updates) in pending.into_iter() {
            updates.sort();
            let mut lines = Vec::new();
            let mut intros = make_hints();

            for mut u in updates {
                u.file_set.sort();
                if let Some(intro) = intros.remove(&u.kind) {
                    if !lines.is_empty() {
                        lines.push(String::new());
                    }
                    lines.push(intro);
                }
                if u.file_set.len() > 10 {
                    lines.push(format!("- {} (>10 files)", u.reference_url));
                } else {
                    lines.push(format!("- {}, files:", u.reference_url));
                    for file in u.file_set {
                        lines.push(format!("  - {}", file));
                    }
                }
            }

            self.github
                .post_comment(full_repo_name, target, lines.join("\n"))
                .await?;
        }
        Ok(())
    }
}

// TODO: add tests
