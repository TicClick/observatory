use viz::IntoResponse;
use viz::{Request, RequestExt, StatusCode};

use crate::{controller, structs};

pub async fn pull_request_event(req: Request, body: String) -> viz::Result<()> {
    let controller_handle = req
        .state::<controller::ControllerHandle>()
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
            controller_handle
                .add_pull(&evt.repository.full_name, evt.pull_request, true)
                .await;
        }
        "closed" => {
            controller_handle
                .remove_pull(&evt.repository.full_name, evt.pull_request)
                .await;
        }
        _ => {}
    }
    Ok(())
}

pub async fn installation_event(req: Request, body: String) -> viz::Result<()> {
    let controller = req
        .state::<controller::ControllerHandle>()
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
            controller.add_installation(evt.installation).await;
        }
        "deleted" => {
            controller.delete_installation(evt.installation).await;
        }
        _ => {}
    }
    Ok(())
}

pub async fn installation_repositories_event(req: Request, body: String) -> viz::Result<()> {
    let controller_handle = req
        .state::<controller::ControllerHandle>()
        .ok_or_else(|| StatusCode::INTERNAL_SERVER_ERROR.into_error())?;

    let evt: structs::InstallationRepositoriesEvent = serde_json::from_str(&body).map_err(|e| {
        log::error!("Failed to deserialize an installation repositories event coming from GitHub: {:?}. JSON: {:?}", e, body);
        StatusCode::INTERNAL_SERVER_ERROR.into_error()
    })?;

    match evt.action.as_str() {
        "added" => {
            controller_handle
                .add_repositories(evt.installation.id, evt.repositories_added)
                .await;
        }
        "removed" => {
            controller_handle
                .remove_repositories(evt.installation.id, evt.repositories_removed)
                .await;
        }
        _ => {}
    }
    Ok(())
}
