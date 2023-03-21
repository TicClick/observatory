mod controller_impl;

use eyre::Result;
use tokio::sync::{mpsc, oneshot};

use crate::config;
use crate::github::GitHubInterface;
use crate::structs::*;

#[derive(Debug)]
pub enum ControllerRequest {
    Init {
        reply_to: oneshot::Sender<Result<()>>,
    },
    GetInstallations {
        reply_to: oneshot::Sender<Vec<Installation>>,
    },
    GetApp {
        reply_to: oneshot::Sender<Option<App>>,
    },

    PullRequestUpdated {
        full_repo_name: String,
        pull_request: Box<PullRequest>,
        trigger_updates: bool,
    },
    PullRequestClosed {
        full_repo_name: String,
        pull_request: Box<PullRequest>,
    },

    InstallationCreated {
        installation: Box<Installation>,
    },
    InstallationDeleted {
        installation: Box<Installation>,
    },

    InstallationRepositoriesAdded {
        installation_id: i64,
        repositories: Vec<Repository>,
    },
    InstallationRepositoriesRemoved {
        installation_id: i64,
        repositories: Vec<Repository>,
    },
}

#[derive(Debug, Clone)]
pub struct ControllerHandle {
    sender: mpsc::Sender<ControllerRequest>,
}

impl ControllerHandle {
    pub fn new<T: GitHubInterface + Sync + Send>(
        app_id: String,
        private_key: String,
        config: config::Controller,
    ) -> Self {
        let (tx, rx) = mpsc::channel(10);
        tokio::spawn(async move {
            controller_impl::Controller::<T>::new(rx, app_id, private_key, config)
                .run_forever()
                .await
        });
        Self { sender: tx }
    }
}

impl ControllerHandle {
    pub async fn init(&self) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        let _ = self
            .sender
            .send(ControllerRequest::Init { reply_to: tx })
            .await;
        rx.await?
    }

    pub async fn get_installations(&self) -> Vec<Installation> {
        let (tx, rx) = oneshot::channel();
        let _ = self
            .sender
            .send(ControllerRequest::GetInstallations { reply_to: tx })
            .await;
        rx.await.unwrap()
    }

    pub async fn get_app(&self) -> Option<App> {
        let (tx, rx) = oneshot::channel();
        let _ = self
            .sender
            .send(ControllerRequest::GetApp { reply_to: tx })
            .await;
        rx.await.unwrap()
    }

    pub async fn add_pull(
        &self,
        full_repo_name: &str,
        pull_request: PullRequest,
        trigger_updates: bool,
    ) {
        let msg = ControllerRequest::PullRequestUpdated {
            full_repo_name: full_repo_name.to_owned(),
            pull_request: Box::new(pull_request),
            trigger_updates,
        };
        self.sender.send(msg).await.unwrap();
    }

    pub async fn remove_pull(&self, full_repo_name: &str, pull_request: PullRequest) {
        let msg = ControllerRequest::PullRequestClosed {
            full_repo_name: full_repo_name.to_owned(),
            pull_request: Box::new(pull_request),
        };
        self.sender.send(msg).await.unwrap();
    }

    pub async fn add_installation(&self, installation: Installation) {
        let msg = ControllerRequest::InstallationCreated {
            installation: Box::new(installation),
        };
        self.sender.send(msg).await.unwrap();
    }

    pub async fn delete_installation(&self, installation: Installation) {
        let msg = ControllerRequest::InstallationDeleted {
            installation: Box::new(installation),
        };
        self.sender.send(msg).await.unwrap();
    }

    pub async fn add_repositories(&self, installation_id: i64, repositories: Vec<Repository>) {
        let msg = ControllerRequest::InstallationRepositoriesAdded {
            installation_id,
            repositories,
        };
        self.sender.send(msg).await.unwrap();
    }

    pub async fn remove_repositories(&self, installation_id: i64, repositories: Vec<Repository>) {
        let msg = ControllerRequest::InstallationRepositoriesRemoved {
            installation_id,
            repositories,
        };
        self.sender.send(msg).await.unwrap();
    }
}
