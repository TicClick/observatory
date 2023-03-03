use viz::IntoResponse;
use viz::{Request, RequestExt, StatusCode};

use crate::{controller, github, structs};

pub async fn pull_request_event(req: Request, body: String) -> viz::Result<()> {
    let controller = req
        .state::<controller::Controller<github::Client>>()
        .ok_or_else(|| StatusCode::INTERNAL_SERVER_ERROR.into_error())?;

    let evt: structs::PullRequestEvent = serde_json::from_str(&body).map_err(|e| {
        log::error!(
            "Failed to deserialize a pull request event coming from GitHub: {:?}. JSON: {:?}",
            e,
            body
        );
        StatusCode::INTERNAL_SERVER_ERROR.into_error()
    })?;

    let pull_number = evt.pull_request.number;
    log::debug!("Pull #{}: received event \"{}\"", pull_number, evt.action);
    match evt.action.as_str() {
        "synchronize" | "opened" | "reopened" => {
            controller
                .add_pull(&evt.repository.full_name, evt.pull_request, true)
                .await
                .unwrap_or_else(|e| {
                    log::error!(
                        "Pull #{}: failed to update information and trigger comments: {:?}",
                        pull_number,
                        e
                    );
                });
        }
        "closed" => {
            controller.remove_pull(&evt.repository.full_name, evt.pull_request);
        }
        _ => {}
    }
    Ok(())
}

pub async fn installation_event(req: Request, body: String) -> viz::Result<()> {
    let controller = req
        .state::<controller::Controller<github::Client>>()
        .ok_or_else(|| StatusCode::INTERNAL_SERVER_ERROR.into_error())?;

    let evt: structs::InstallationEvent = serde_json::from_str(&body).map_err(|e| {
        log::error!("Failed to deserialize an installation request event coming from GitHub: {:?}. JSON: {:?}", e, body);
        StatusCode::INTERNAL_SERVER_ERROR.into_error()
    })?;

    let installation_id = evt.installation.id;
    log::debug!(
        "Installation #{}: received event \"{}\"",
        installation_id,
        evt.action
    );
    match evt.action.as_str() {
        "created" => {
            controller
                .add_installation(evt.installation)
                .await
                .unwrap_or_else(|e| {
                    log::error!(
                        "Installation #{}: addition failed (owner: {}, repositories: {:?}): {:?}",
                        installation_id,
                        evt.sender.login,
                        evt.repositories
                            .iter()
                            .map(|r| r.full_name.clone())
                            .collect::<Vec<String>>(),
                        e
                    );
                });
        }
        "deleted" => {
            controller.remove_installation(evt.installation);
        }
        _ => {}
    }
    Ok(())
}

pub async fn installation_repositories_event(req: Request, body: String) -> viz::Result<()> {
    let controller = req
        .state::<controller::Controller<github::Client>>()
        .ok_or_else(|| StatusCode::INTERNAL_SERVER_ERROR.into_error())?;

    let evt: structs::InstallationRepositoriesEvent = serde_json::from_str(&body).map_err(|e| {
        log::error!("Failed to deserialize an installation repositories event coming from GitHub: {:?}. JSON: {:?}", e, body);
        StatusCode::INTERNAL_SERVER_ERROR.into_error()
    })?;

    // TODO: this should be more elegant
    let mut cached_installations = controller.installations();
    for inst in cached_installations.iter_mut() {
        if inst.id == evt.installation.id {
            let removed_repos: Vec<_> = evt.repositories_removed.iter().map(|r| r.id).collect();
            let mut i = 0;
            while i < inst.repositories.len() {
                if removed_repos.contains(&inst.repositories[i].id) {
                    inst.repositories.remove(i);
                } else {
                    i += 1;
                }
            }
            inst.repositories
                .append(&mut evt.repositories_added.clone());
            controller.update_cached_installation(inst.clone());
            break;
        }
    }
    for repo in evt.repositories_added {
        if let Err(e) = controller.add_repository(&repo).await {
            log::error!(
                "Failed to handle addition of repository {:?}: {:?}",
                repo,
                e
            );
        }
    }
    for repo in evt.repositories_removed {
        log::debug!("Removing repository {:?}", repo);
        controller.remove_repository(&repo);
    }
    Ok(())
}
