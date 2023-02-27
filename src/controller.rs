/// `controller` contains core logic of the app. Refer to [`Controller`] for more details.
use std::collections::HashMap;

use eyre::Result;

use crate::helpers::comments::{CommentHeader, ToMarkdown};
use crate::helpers::pulls::{self, ConflictType};
use crate::structs::IssueComment;
use crate::{github, memory, structs};

/// Controller is a representation of a GitHub App, which contains a per-repository cache of
/// pull requests and corresponding `.diff` files.
///
/// The controller handles pull request updates and maintains the cache accordingly. After initialization,
/// it is only aware of available repositories and current state of pull requests -- updates need to be passed by the controller owner.
///
// The controller checks incoming updates against memory and attempts to determine whether there are conflicts on article levels.
/// (for details, see [`ConflictType`]). After that, it leaves comments on the pull request which depends on the changes; typically, that is
/// a translation, whose owner needs to be made aware of changes they may be missing.
#[derive(Debug, Clone)]
pub struct Controller {
    /// Information about a GitHub app (used to detect own comments).
    pub app: Option<structs::App>,

    /// GitHub API client -- see [`github::Client`] for details.
    github: github::Client,

    /// The cache with pull requests and their diffs.
    memory: memory::Memory,
}

impl Controller {
    pub fn new(app_id: String, private_key: String) -> Self {
        Self {
            app: None,
            github: github::Client::new(app_id, private_key),
            memory: memory::Memory::new(),
        }
    }

    /// Fetch a list of installations. Installations generally correspond to GitHub repositories,
    /// for which the controller will receive updates.
    pub fn installations(&self) -> Vec<structs::Installation> {
        self.github
            .installations
            .lock()
            .unwrap()
            .values()
            .cloned()
            .collect()
    }

    /// Build the in-memory pull request cache on start-up. This will consume a lot of GitHub API quota,
    /// but fighting a stale database cache is left as an exercise for another day.
    pub async fn init(&mut self) -> Result<()> {
        self.app = Some(self.github.app().await?);
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

    /// Add an installation and fetch pull requests (one installation may have several repos).
    pub async fn add_installation(&self, installation: structs::Installation) -> Result<()> {
        self.github.add_installation(installation.clone()).await?;
        for r in installation.repositories {
            for p in self.github.pulls(&r.full_name).await? {
                self.add_pull(&r.full_name, p, false).await?;
            }
        }
        Ok(())
    }

    /// Remove an installation from cache and forget about its pull requests.
    pub fn remove_installation(&self, installation: structs::Installation) {
        self.github.remove_installation(&installation);
        for r in installation.repositories {
            self.memory.drop_repository(&r.full_name);
        }
    }

    /// Purge a pull request from memory, excluding it from conflict detection.
    ///
    /// This should be done only when a pull request is closed or merged.
    pub fn remove_pull(&self, full_repo_name: &str, closed_pull: structs::PullRequest) {
        self.memory.remove_pull(full_repo_name, &closed_pull);
    }

    /// Handle pull request changes. This includes fetching a `.diff` file from another GitHub domain,
    /// which may have its own rate limits.
    ///
    /// If `trigger_updates` is set, check if the update conflicts with existing pull requests,
    /// and make its author aware (or other PRs' owners, in rare cases).
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
        self.memory.insert_pull(full_repo_name, new_pull.clone());

        let mut pending_updates: HashMap<i32, Vec<pulls::Conflict>> = HashMap::new();
        if let Some(pulls_map) = self.memory.pulls(full_repo_name) {
            let mut pulls: Vec<structs::PullRequest> = pulls_map
                .into_values()
                .filter(|other| other.number != new_pull.number)
                .collect();
            pulls.sort_by_key(|pr| pr.created_at);

            // Compare the new pull with existing for conflicts.
            // Known conflicts are skipped (same kind + same file set), otherwise memory is updated.
            let mut existing_conflicts = self.memory.conflicts(full_repo_name);
            for other_pull in pulls {
                let conflicts = pulls::compare_pulls(&new_pull, &other_pull);
                for conflict in conflicts {
                    let mut skip_commenting = false;
                    if let Some(ec) = existing_conflicts.get_mut(&conflict.notification_target) {
                        for i in ec.iter_mut() {
                            if i.reference_target == conflict.reference_target && i.kind == conflict.kind {
                                if i.file_set == conflict.file_set  {
                                    skip_commenting = true;
                                }
                                i.file_set = conflict.file_set.clone();
                                break;
                            }
                        }
                    }
                    if skip_commenting {
                        continue;
                    }
                    pending_updates
                        .entry(conflict.notification_target)
                        .or_default()
                        .push(conflict);
                }
            }
            self.memory.replace_conflicts(full_repo_name, existing_conflicts);
        }
        if !trigger_updates {
            return Ok(());
        }
        self.send_updates(pending_updates, full_repo_name).await?;
        Ok(())
    }

    /// Notify pull request authors about conflicts by sending a comment for every
    /// `(conflict source, conflict type)` combination.
    ///
    /// Every comment contains a machine-readable YAML header, hidden between separate HTML comment tags.
    /// The header is a reliable alternative to parsing everything from comments (provided no one tampers with them).
    ///
    /// Comments already left by the bot are reused for updates, both to avoid spam and make notification process easier.
    pub async fn send_updates(
        &self,
        pending: HashMap<i32, Vec<pulls::Conflict>>,
        full_repo_name: &str,
    ) -> Result<()> {
        for (target, updates) in pending.into_iter() {
            let existing_comments = self
                .github
                .list_comments(full_repo_name, target)
                .await?
                .into_iter()
                .filter(|c| self.has_control_over(&c.user));
            let mut pull_references: HashMap<(i32, ConflictType), IssueComment> = HashMap::new();
            for c in existing_comments {
                if let Some(header) = CommentHeader::from_comment(&c.body) {
                    pull_references.insert((header.pull_number, header.conflict_type), c);
                }
            }

            for u in updates {
                let key = (u.reference_target, u.kind.clone());
                if let Some(existing_comment) = pull_references.get(&key) {
                    if let Err(e) = self
                        .github
                        .update_comment(full_repo_name, existing_comment.id, u.to_markdown())
                        .await
                    {
                        log::error!(
                            "Failed to update comment #{} in pull request {}/#{} (about #{}): {:?}",
                            existing_comment.id,
                            full_repo_name,
                            key.0,
                            target,
                            e
                        );
                    }
                } else if let Err(e) = self
                    .github
                    .post_comment(full_repo_name, target, u.to_markdown())
                    .await
                {
                    log::error!(
                        "Failed to post a NEW comment in pull request {}/#{} (about #{}): {:?}",
                        full_repo_name,
                        key.0,
                        target,
                        e
                    );
                }
            }
        }
        Ok(())
    }

    /// A helper for checking if the comment is made by the bot itself.
    ///
    /// Curiously, there is no way of telling this from the comment's JSON.
    fn has_control_over(&self, user: &structs::Actor) -> bool {
        if let Some(app) = &self.app {
            user.login == format!("{}[bot]", &app.slug)
        } else {
            false
        }
    }
}

// TODO: add tests
