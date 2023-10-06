/// `controller` contains core logic of the app. Refer to [`Controller`] for more details.
use std::collections::HashMap;

use eyre::Result;
use tokio::sync::mpsc;

use crate::config;
use crate::controller::ControllerRequest;
use crate::github::{GitHub, GitHubInterface};
use crate::helpers::comments::CommentHeader;
use crate::helpers::conflicts::{self, ConflictType};
use crate::helpers::ToMarkdown;
use crate::memory;
use crate::structs::*;

/// Controller is a representation of a GitHub App, which contains a per-repository cache of
/// pull requests and corresponding `.diff` files. It is used from the facade, [`super::ControllerHandle`].
///
/// The controller handles pull request updates and maintains the cache accordingly. After initialization,
/// it is only aware of available repositories and current state of pull requests -- updates need to be passed by the controller owner.
///
// The controller checks incoming updates against memory and attempts to determine whether there are conflicts on article levels.
/// (for details, see [`ConflictType`]). After that, it leaves comments on the pull request which depends on the changes; typically, that is
/// a translation, whose owner needs to be made aware of changes they may be missing.
#[derive(Debug)]
pub(super) struct Controller<T>
where
    T: GitHubInterface,
{
    /// The event queue with requests coming from the controller handle.
    receiver: mpsc::Receiver<ControllerRequest>,

    /// Information about a GitHub app (used to detect own comments).
    app: Option<App>,

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
    /// Start processing events one at a time. This function blocks until the receiver is destroyed, which happens
    /// on handle destruction automatically.
    pub(super) async fn run_forever(&mut self) {
        while let Some(msg) = self.receiver.recv().await {
            self.handle_message(msg).await;
        }
    }

    /// Dispatch the message from a handle to an appropriate method, and possibly return the call result.
    async fn handle_message(&mut self, message: ControllerRequest) {
        match message {
            ControllerRequest::Init { reply_to } => {
                reply_to.send(self.init().await).unwrap();
            }

            ControllerRequest::PullRequestCreated {
                full_repo_name,
                pull_request,
                trigger_updates,
            } => {
                let pull_number = pull_request.number;
                self.add_pull(&full_repo_name, *pull_request, trigger_updates)
                    .await
                    .unwrap_or_else(|e| {
                        log::error!(
                            "Pull #{}: failed to add information and trigger comments: {:?}",
                            pull_number,
                            e
                        );
                    })
            }

            ControllerRequest::PullRequestUpdated {
                full_repo_name,
                pull_request,
                trigger_updates,
            } => {
                let pull_number = pull_request.number;
                self.update_pull(&full_repo_name, *pull_request, trigger_updates)
                    .await
                    .unwrap_or_else(|e| {
                        log::error!(
                            "Pull #{}: failed to update information and trigger comments: {:?}",
                            pull_number,
                            e
                        );
                    })
            }
            ControllerRequest::PullRequestClosed {
                full_repo_name,
                pull_request,
            } => {
                self.remove_pull(&full_repo_name, *pull_request);
            }

            ControllerRequest::InstallationCreated { installation } => {
                let iid = installation.id;
                self.add_installation(*installation)
                    .await
                    .unwrap_or_else(|e| {
                        log::error!("Installation #{}: addition failed: {:?}", iid, e);
                    });
            }
            ControllerRequest::InstallationDeleted { installation } => {
                self.delete_installation(*installation)
            }

            ControllerRequest::InstallationRepositoriesAdded {
                installation_id,
                repositories,
            } => {
                self.add_repositories(installation_id, repositories).await;
            }

            ControllerRequest::InstallationRepositoriesRemoved {
                installation_id,
                repositories,
            } => {
                self.remove_repositories(installation_id, &repositories);
            }
        }
    }

    /// Create an unitialized controller.
    pub(super) fn new(
        receiver: mpsc::Receiver<ControllerRequest>,
        app_id: String,
        private_key: String,
        config: config::Controller,
    ) -> Self {
        Self {
            receiver,
            app: None,
            github: T::new(app_id, private_key),
            memory: memory::Memory::new(),
            conflicts: conflicts::Storage::default(),
            config,
        }
    }

    /// Build the in-memory pull request cache on start-up. This will consume a lot of GitHub API quota,
    /// but fighting a stale database cache is left as an exercise for another day.
    async fn init(&mut self) -> Result<()> {
        self.app = Some(self.github.app().await?);
        log::info!("GitHub application: {:?}", self.app.as_ref().unwrap());

        let installations = self.github.discover_installations().await?;
        log::info!("Active installations: {:?}", installations);
        for i in installations {
            self.add_repositories(i.id, self.github.cached_repositories(i.id))
                .await;
        }
        Ok(())
    }

    /// Add an installation and fetch pull requests (one installation may have several repos).
    async fn add_installation(&self, installation: Installation) -> Result<()> {
        let iid = installation.id;
        self.github.add_installation(installation).await?;
        self.add_repositories(iid, self.github.cached_repositories(iid))
            .await;
        Ok(())
    }

    /// Add several repositories the app just got an access to.
    async fn add_repositories(&self, installation_id: i64, repositories: Vec<Repository>) {
        self.github
            .add_repositories(installation_id, repositories.clone());
        for r in repositories {
            log::debug!(
                "Adding repository {:?} for installation #{}",
                r,
                installation_id
            );
            self.add_repository(&r).await.unwrap_or_else(|e| {
                log::error!(
                    "Repository {:?} for installation #{}: addition failed: {:?}",
                    r,
                    installation_id,
                    e
                );
            });
        }
    }

    /// Add a repository and fetch its pull requests.
    async fn add_repository(&self, r: &Repository) -> Result<()> {
        for p in self.github.pulls(&r.full_name).await? {
            self.add_pull(&r.full_name, p, false).await?;
        }
        Ok(())
    }

    /// Remove an installation from cache and forget about its pull requests.
    fn delete_installation(&self, installation: Installation) {
        let repos = self.github.cached_repositories(installation.id);
        self.github.remove_installation(&installation);
        self.remove_repositories(installation.id, &repos);
    }

    /// Remove muliple repositories which the app has just lost its access to.
    fn remove_repositories(&self, installation_id: i64, repositories: &[Repository]) {
        for r in repositories {
            log::debug!(
                "Removing repository {:?} for installation #{}",
                r,
                installation_id
            );
            self.memory.drop_repository(&r.full_name);
            self.conflicts.remove_repository(&r.full_name);
        }
        self.github
            .remove_repositories(installation_id, repositories);
    }

    /// Purge a pull request from memory, excluding it from conflict detection.
    ///
    /// This should be done only when a pull request is closed or merged.
    fn remove_pull(&self, full_repo_name: &str, closed_pull: PullRequest) {
        self.memory.remove_pull(full_repo_name, &closed_pull);
        self.conflicts
            .remove_conflicts_by_pull(full_repo_name, closed_pull.number);
    }

    async fn update_pull(
        &self,
        full_repo_name: &str,
        new_pull: PullRequest,
        trigger_updates: bool,
    ) -> Result<()> {
        if self.memory.contains(full_repo_name, &new_pull) {
            self.add_pull(full_repo_name, new_pull, trigger_updates)
                .await
        } else {
            log::info!(
                "Pull #{} can't be updated because it wasn't added in the first place",
                new_pull.number
            );
            Ok(())
        }
    }

    /// Handle pull request changes. This includes fetching a `.diff` file from another GitHub domain,
    /// which may have its own rate limits.
    ///
    /// If `trigger_updates` is set, check if the update conflicts with existing pull requests,
    /// and make its author aware (or other PRs' owners, in rare cases). For details, see [`helpers::conflicts::Storage`].
    async fn add_pull(
        &self,
        full_repo_name: &str,
        mut new_pull: PullRequest,
        trigger_updates: bool,
    ) -> Result<()> {
        let diff = self
            .github
            .read_pull_diff(full_repo_name, new_pull.number)
            .await?;
        new_pull.diff = Some(diff);
        self.memory.insert_pull(full_repo_name, new_pull.clone());

        if let Some(pulls_map) = self.memory.pulls(full_repo_name) {
            let mut pulls: Vec<PullRequest> = pulls_map
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
    async fn send_updates(
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
    fn has_control_over(&self, user: &Actor) -> bool {
        if let Some(app) = &self.app {
            user.login == format!("{}[bot]", &app.slug)
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests;
