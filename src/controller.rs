/// `controller` contains core logic of the app. Refer to [`Controller`] for more details.
use std::collections::HashMap;

use eyre::Result;

use crate::config;
use crate::github::{GitHub, GitHubInterface};
use crate::helpers::comments::CommentHeader;
use crate::helpers::conflicts::{self, ConflictType};
use crate::helpers::ToMarkdown;
use crate::structs::IssueComment;
use crate::{memory, structs};

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
pub struct Controller<T>
where
    T: GitHubInterface,
{
    /// Information about a GitHub app (used to detect own comments).
    pub app: Option<structs::App>,

    /// GitHub API client -- see [`github::Client`] for details.
    github: T,

    /// The cache with pull requests and their diffs.
    memory: memory::Memory,

    /// The conflicts cache for continuous update.
    conflicts: conflicts::Storage,

    /// Controller-specific settings taken from `config.yaml`.
    config: config::Controller,
}

impl<T: GitHubInterface> Controller<T> {
    pub fn new(app_id: String, private_key: String, config: config::Controller) -> Self {
        Self {
            app: None,
            github: T::new(app_id, private_key),
            memory: memory::Memory::new(),
            conflicts: conflicts::Storage::default(),
            config,
        }
    }

    /// Obtain list of current GitHub App installations and their repositories.
    pub fn installations(&self) -> Vec<structs::Installation> {
        self.github.cached_installations()
    }

    /// Update list of current GitHub App installations and their repositories after handling an update event.
    pub fn update_cached_installation(&self, installation: structs::Installation) {
        self.github.update_cached_installation(installation);
    }

    /// Build the in-memory pull request cache on start-up. This will consume a lot of GitHub API quota,
    /// but fighting a stale database cache is left as an exercise for another day.
    pub async fn init(&mut self) -> Result<()> {
        self.app = Some(self.github.app().await?);
        let installations = self.github.discover_installations().await?;
        for i in installations {
            for r in i.repositories {
                self.add_repository(&r).await?;
            }
        }
        Ok(())
    }

    /// Add an installation and fetch pull requests (one installation may have several repos).
    pub async fn add_installation(&self, installation: structs::Installation) -> Result<()> {
        let updated_installation = self.github.add_installation(installation).await?;
        for r in updated_installation.repositories {
            self.add_repository(&r).await?;
        }
        Ok(())
    }

    /// Add a repository and fetch its pull requests.
    pub async fn add_repository(&self, r: &structs::Repository) -> Result<()> {
        for p in self.github.pulls(&r.full_name).await? {
            self.add_pull(&r.full_name, p, false).await?;
        }
        Ok(())
    }

    /// Remove an installation from cache and forget about its pull requests.
    pub fn remove_installation(&self, installation: structs::Installation) {
        self.github.remove_installation(&installation);
        for r in installation.repositories {
            self.remove_repository(&r);
        }
    }

    /// Remove repository from memory, forgetting anything about it.
    pub fn remove_repository(&self, r: &structs::Repository) {
        self.memory.drop_repository(&r.full_name);
        self.conflicts.remove_repository(&r.full_name)
    }

    /// Purge a pull request from memory, excluding it from conflict detection.
    ///
    /// This should be done only when a pull request is closed or merged.
    pub fn remove_pull(&self, full_repo_name: &str, closed_pull: structs::PullRequest) {
        self.memory.remove_pull(full_repo_name, &closed_pull);
        self.conflicts
            .remove_conflicts_by_pull(full_repo_name, closed_pull.number);
    }

    /// Handle pull request changes. This includes fetching a `.diff` file from another GitHub domain,
    /// which may have its own rate limits.
    ///
    /// If `trigger_updates` is set, check if the update conflicts with existing pull requests,
    /// and make its author aware (or other PRs' owners, in rare cases). For details, see [`helpers::conflicts::Storage`].
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

        if let Some(pulls_map) = self.memory.pulls(full_repo_name) {
            let mut pulls: Vec<structs::PullRequest> = pulls_map
                .into_values()
                .filter(|other| other.number != new_pull.number)
                .collect();
            pulls.sort_by_key(|pr| pr.created_at);

            // Compare the new pull with existing ones for conflicts:
            // - Known conflicts (same kind + same file set) are skipped, otherwise memory is updated.
            // - Conflicts that don't occur anymore are removed from cache, with subsequent comment removal.

            let mut pending_updates: HashMap<i32, Vec<conflicts::Conflict>> = HashMap::new();
            let mut conflicts_to_remove: HashMap<i32, Vec<conflicts::Conflict>> = HashMap::new();
            for other_pull in pulls {
                let conflicts = conflicts::compare_pulls(&new_pull, &other_pull);

                // Note: after a conflict disappears, any interfering updates to the original pull will flip the roles:
                // the pull which triggered the new conflict will be considered an original. This is a scenario rare enough
                // (think indecisive people bringing changes in and out), but one that we should consider and have written down.
                // Also, it's simpler than maintaining a cache of "inactive" conflicts, at least for now.
                // Related test: test_new_comment_is_posted_after_removal_in_different_pull

                let removed_conflicts = self.conflicts.remove_missing(
                    full_repo_name,
                    other_pull.number,
                    new_pull.number,
                    &conflicts,
                );
                for removed in removed_conflicts {
                    conflicts_to_remove
                        .entry(removed.trigger)
                        .or_default()
                        .push(removed);
                }

                for conflict in conflicts {
                    if let Some(updated_conflict) = self.conflicts.upsert(full_repo_name, &conflict)
                    {
                        pending_updates
                            .entry(updated_conflict.trigger)
                            .or_default()
                            .push(updated_conflict);
                    }
                }
            }
            if trigger_updates {
                self.send_updates(pending_updates, conflicts_to_remove, full_repo_name)
                    .await?;
            }
        }
        Ok(())
    }

    /// Notify pull request authors about conflicts by sending a comment for every
    /// `(conflict source, conflict type)` combination.
    ///
    /// Every comment contains a machine-readable YAML header, hidden between separate HTML comment tags.
    /// The header is a reliable alternative to parsing everything from comments (provided no one tampers with them).
    ///
    /// Comments already left by the bot are reused for updates, both to avoid spam and make notification process easier.
    /// Comments about obsolete conflicts are removed; the lists of conflicts to update and to remove have no intersection.
    pub async fn send_updates(
        &self,
        pending: HashMap<i32, Vec<conflicts::Conflict>>,
        to_remove: HashMap<i32, Vec<conflicts::Conflict>>,
        full_repo_name: &str,
    ) -> Result<()> {
        // Read all comments in affected pulls and find these which point to other pulls ("originals").
        let mut pull_references: HashMap<(i32, ConflictType), IssueComment> = HashMap::new();
        for pull_number in pending.keys().chain(to_remove.keys()) {
            let existing_comments = self
                .github
                .list_comments(full_repo_name, *pull_number)
                .await?
                .into_iter()
                .filter(|c| self.has_control_over(&c.user));
            for c in existing_comments {
                if let Some(header) = CommentHeader::from_comment(&c.body) {
                    pull_references.insert((header.pull_number, header.conflict_type), c);
                }
            }
        }

        for (pull_to_clean, obsolete_conflicts) in to_remove.into_iter() {
            for r in obsolete_conflicts {
                let key = (r.original, r.kind.clone());
                if let Some(existing_comment) = pull_references.get(&key) {
                    if self.config.post_comments {
                        if let Err(e) = self
                            .github
                            .delete_comment(full_repo_name, existing_comment.id)
                            .await
                        {
                            log::error!(
                                "Failed to delete comment #{} about pull #{} of kind {:?} in {}: {:?}",
                                existing_comment.id,
                                r.original,
                                r.kind,
                                GitHub::pull_url(full_repo_name, pull_to_clean),
                                e
                            );
                        } else {
                            log::debug!(
                                "Would delete comment #{} about pull #{} of kind {:?} in {}",
                                existing_comment.id,
                                r.original,
                                r.kind,
                                GitHub::pull_url(full_repo_name, pull_to_clean),
                            );
                        }
                    }
                }
            }
        }

        for (pull_to_notify, updates) in pending.into_iter() {
            for u in updates {
                let key = (u.original, u.kind.clone());
                if let Some(existing_comment) = pull_references.get(&key) {
                    if self.config.post_comments {
                        if let Err(e) = self
                            .github
                            .update_comment(full_repo_name, existing_comment.id, u.to_markdown())
                            .await
                        {
                            log::error!(
                                "Failed to update comment #{} about pull #{} of kind {:?} in {}: {:?}",
                                existing_comment.id,
                                u.original,
                                u.kind,
                                GitHub::pull_url(full_repo_name, pull_to_notify),
                                e
                            );
                        }
                    } else {
                        log::debug!(
                            "Would update comment #{} about pull #{} of kind {:?} in {}",
                            existing_comment.id,
                            u.original,
                            u.kind,
                            GitHub::pull_url(full_repo_name, pull_to_notify),
                        );
                    }
                } else if self.config.post_comments {
                    if let Err(e) = self
                        .github
                        .post_comment(full_repo_name, pull_to_notify, u.to_markdown())
                        .await
                    {
                        log::error!(
                            "Failed to post a NEW comment about pull #{} of kind {:?} in {}: {:?}",
                            u.original,
                            u.kind,
                            GitHub::pull_url(full_repo_name, pull_to_notify),
                            e
                        );
                    }
                } else {
                    log::debug!(
                        "Would post a NEW comment about #{} of kind {:?} in {}",
                        u.original,
                        u.kind,
                        GitHub::pull_url(full_repo_name, pull_to_notify),
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

#[cfg(test)]
#[path = "controller_test.rs"]
pub(crate) mod tests;
