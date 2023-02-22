use viz::IntoResponse;
use viz::{Request, RequestExt, StatusCode};

use crate::{controller, structs};

pub async fn pull_request_event(req: Request, body: String) -> viz::Result<()> {
    let controller = req
        .state::<controller::Controller>()
        .ok_or_else(|| StatusCode::INTERNAL_SERVER_ERROR.into_error())?;

    let evt: structs::PullRequestEvent = serde_json::from_str(&body).map_err(|e| {
        log::error!("Failed to deserialize an event coming from GitHub: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR.into_error()
    })?;

    match evt.action.as_str() {
        "synchronize" | "opened" | "reopened" => {
            let pull_number = evt.pull_request.number;
            controller
                .add_pull(&evt.repository.full_name, evt.pull_request, true)
                .await
                .unwrap_or_else(|e| {
                    log::error!(
                        "Failed to update information about pull #{}: {:?}",
                        pull_number,
                        e
                    );
                });
        }
        "closed" => {
            controller
                .remove_pull(&evt.repository.full_name, evt.pull_request)
                .await;
        }
        _ => {}
    }
    Ok(())
}

pub async fn installation_event(req: Request, body: String) -> viz::Result<()> {
    let controller = req
        .state::<controller::Controller>()
        .ok_or_else(|| StatusCode::INTERNAL_SERVER_ERROR.into_error())?;

    let evt: structs::InstallationEvent = serde_json::from_str(&body).map_err(|e| {
        log::error!("Failed to deserialize an event coming from GitHub: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR.into_error()
    })?;

    match evt.action.as_str() {
        "created" => {
            controller
                .add_installation(evt.installation)
                .await
                .unwrap_or_else(|e| {
                    log::error!(
                        "Failed to add an installation (owner: {}, repositories: {:?}): {:?}",
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
