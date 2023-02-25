// TODO: document members of the module where it makes sense

use std::net::SocketAddr;

use clap::Parser;
use eyre::Result;

use viz::middleware::limits;
use viz::IntoResponse;
use viz::{types::State, Router, Server, ServiceMaker};
use viz::{Request, RequestExt, StatusCode};

use observatory::{config, controller, handler};

#[derive(Parser, Debug)]
#[command(version)]
struct Args {
    /// Path to settings.yaml. Omit to search in current working directory
    #[arg(short, long, default_value_t = config::DEFAULT_FILE_NAME.to_string())]
    config: String,
}

pub async fn index(_: Request) -> viz::Result<String> {
    Ok(r"¯\_(ツ)_/¯".to_owned())
}

#[derive(Debug, Clone)]
pub struct RequestValidator {
    token: String,
}

impl RequestValidator {
    pub fn new(token: String) -> Self {
        Self { token }
    }

    pub fn validate(&self, data: &str, signature: &str) -> Result<bool> {
        // The signature is a string where every two letters describe a byte (high 4 bits | low 4 bits).
        // Example: fbf26f84aa96ef919cb4b2d81e86cb9236204e48d16b6e34145d36acfd0b8d5d
        // To convert it to bytes, read every letter as a hex digit, then combine every pair of them
        // into a proper byte.
        let from_hex: Vec<u8> = signature
            .chars()
            .map(|ch| match ch {
                '0'..='9' => ch as u8 - b'0',
                'a'..='f' => (ch as u8 - b'a') + 10,
                _ => b'_', // yeah this isn't supposed to happen
            })
            .collect();
        let mut bytes = Vec::new();
        for i in 0..from_hex.len() / 2 {
            bytes.push(from_hex[2 * i] << 4 | from_hex[2 * i + 1]);
        }

        let key = &ring::hmac::Key::new(ring::hmac::HMAC_SHA256, self.token.as_bytes());
        let local_signature = ring::hmac::sign(key, data.as_bytes());
        Ok(bytes == local_signature.as_ref())
    }
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

    // TODO: instead of processing requests right here, use std::sync::mspc channels -- this way we won't even need to
    // access the controller from the web server.
    match event_type.as_str() {
        "pull_request" => handler::pull_request_event(req, body).await,
        "installation" => handler::installation_event(req, body).await,
        // TODO: handle "installation_repositories" events, similarly to the above. This is nice to have, but not a necessity,
        // since we retain installation tokens even if a repository is removed from us.
        // https://docs.github.com/webhooks-and-events/webhooks/webhook-events-and-payloads#installation_repositories
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
    let mut controller = controller::Controller::new(settings.github.app_id, private_key);
    controller.init().await?;
    log::info!("Active installations: {:?}", controller.installations());
    log::debug!("GitHub App: {:?}", controller.app);

    let ls = viz::types::Limits::new()
        .insert("bytes", DEFAULT_DATA_LIMIT)
        .insert("json", DEFAULT_DATA_LIMIT)
        .insert("payload", DEFAULT_DATA_LIMIT)
        .insert("text", DEFAULT_DATA_LIMIT);

    let app = Router::new()
        .post(&settings.server.events_endpoint, github_events)
        .get("/", index)
        .with(State::new(controller))
        .with(State::new(validator))
        .with(limits::Config::default().limits(ls));

    log::info!("Listening on {}/{}", addr, settings.server.events_endpoint);
    if let Err(err) = Server::bind(&addr).serve(ServiceMaker::from(app)).await {
        log::error!("{:?}", err);
    }

    Ok(())
}

// TODO: add tests for event processing?
