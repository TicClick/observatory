// TODO: document members of the module where it makes sense

use std::net::SocketAddr;

use clap::Parser;
use eyre::Result;

use viz::middleware::limits;
use viz::{types::State, Router, Server, ServiceMaker};
use viz::{IntoResponse, Response, ResponseExt};
use viz::{Request, RequestExt, StatusCode};

use observatory::helpers::digest::RequestValidator;
use observatory::{config, controller, github, handler, helpers::cgroup};

#[derive(Parser, Debug)]
#[command(version)]
struct Args {
    /// Path to settings.yaml. Omit to search in current working directory
    #[arg(short, long, default_value_t = config::DEFAULT_FILE_NAME.to_string())]
    config: String,
}

pub async fn index(_: Request) -> viz::Result<Response> {
    if !cfg!(windows) {
        let mut body = Vec::new();
        for (header, value) in cgroup::CGroup::current().summary() {
            let val = value.replace('\n', "<br/>");
            body.push(format!(r"<h3><tt>{header}</tt></h3><tt>{val}</tt><br/>"));
        }
        return Ok(Response::html(body.join("")));
    }
    Ok(Response::html(r"¯\_(ツ)_/¯".to_owned()))
}

pub async fn github_events(mut req: Request) -> viz::Result<()> {
    let event_type = req.header::<_, String>("X-GitHub-Event").ok_or_else(|| {
        log::warn!("GitHub event is missing the event type header, rejecting");
        StatusCode::FORBIDDEN.into_error()
    })?;

    let signature_header = req
        .header::<_, String>("X-Hub-Signature-256")
        .ok_or_else(|| {
            log::warn!("GitHub event is missing the signature header, rejecting");
            StatusCode::FORBIDDEN.into_error()
        })?;
    let signature = &signature_header.strip_prefix("sha256=").unwrap();

    let body = req.text().await?;
    let validator = req
        .state::<RequestValidator>()
        .ok_or_else(|| StatusCode::INTERNAL_SERVER_ERROR.into_error())?;
    if !validator.validate(&body, signature).unwrap() {
        return Err(StatusCode::FORBIDDEN.into_error());
    }

    match event_type.as_str() {
        "pull_request" => handler::pull_request_event(req, body).await,
        "installation" => handler::installation_event(req, body).await,
        "installation_repositories" => handler::installation_repositories_event(req, body).await,
        _ => Ok(()),
    }
}

const DEFAULT_DATA_LIMIT: u64 = 10 * 1024 * 1024; // 10 Mb

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let settings = config::Config::from_path(&args.config)?;
    let addr = SocketAddr::from((settings.server.bind_ip, settings.server.port));

    let logging_config = simplelog::ConfigBuilder::new()
        .set_time_format_custom(simplelog::format_description!(
            "[year]-[month]-[day] [hour]:[minute]:[second].[subsecond]"
        ))
        .build();
    if settings.logging.file == config::STDERR_LOG_FILE {
        simplelog::TermLogger::init(
            settings.logging.level,
            logging_config,
            simplelog::TerminalMode::Stderr,
            simplelog::ColorChoice::Auto,
        )
        .expect("Failed to configure the terminal logger");
    } else {
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(settings.logging.file)
            .expect("Failed to open the log file -- check CLI arguments");
        simplelog::WriteLogger::init(settings.logging.level, logging_config, file)
            .expect("Failed to configure the file logger");
    }

    log_panics::init();
    log::info!("----- Starting up...");

    let private_key = std::fs::read_to_string(std::path::Path::new(&settings.github.app_key_path))
        .expect("Failed to read GitHub App private key");
    let webhook_secret = settings.github.webhook_secret;

    let validator = RequestValidator::new(webhook_secret);
    let controller_handle = controller::ControllerHandle::new::<github::Client>(
        settings.github.app_id,
        private_key,
        settings.controller.clone(),
    );
    controller_handle.init().await?;

    let ls = viz::types::Limits::new()
        .insert("bytes", DEFAULT_DATA_LIMIT)
        .insert("json", DEFAULT_DATA_LIMIT)
        .insert("payload", DEFAULT_DATA_LIMIT)
        .insert("text", DEFAULT_DATA_LIMIT);

    let app = Router::new()
        .post(&settings.server.events_endpoint, github_events)
        .get("/", index)
        .with(State::new(controller_handle))
        .with(State::new(validator))
        .with(limits::Config::default().limits(ls));

    log::info!("Listening on {}/{}", addr, settings.server.events_endpoint);
    if let Err(err) = Server::bind(&addr).serve(ServiceMaker::from(app)).await {
        log::error!("{:?}", err);
    }

    Ok(())
}

// TODO: add tests for event processing?
