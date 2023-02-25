/// The config module stores structs for working with a YAML config file.
///
/// To avoid silently using unexpected defaults, all values must be defined only in the YAML file.
/// For detailed information on what every setting does, refer to `.config.yaml`.
use std::net::Ipv4Addr;

use eyre::Result;
use serde::{Deserialize, Serialize};

pub const DEFAULT_FILE_NAME: &str = "config.yaml";
pub const STDERR_LOG_FILE: &str = "-";

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Config {
    pub server: Server,
    pub logging: Logging,
    pub github: GitHub,
}

impl Config {
    pub fn from_path(path: &str) -> Result<Config> {
        let contents = std::fs::read_to_string(path)?;
        let settings = serde_yaml::from_str::<Config>(contents.as_str())?;
        Ok(settings)
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Server {
    pub bind_ip: Ipv4Addr,
    pub port: u16,
    pub events_endpoint: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Logging {
    pub file: String,

    #[serde(with = "LevelFilterDef")]
    pub level: log::LevelFilter,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct GitHub {
    pub app_id: String,
    pub app_key_path: String,
    pub webhook_secret: String,
}

// Unfortunate copypaste: https://serde.rs/remote-derive.html
#[derive(Serialize, Deserialize, Debug)]
#[serde(remote = "log::LevelFilter")]
enum LevelFilterDef {
    #[serde(alias = "off")]
    Off,
    #[serde(alias = "error")]
    Error,
    #[serde(alias = "warn")]
    Warn,
    #[serde(alias = "info")]
    Info,
    #[serde(alias = "debug")]
    Debug,
    #[serde(alias = "trace")]
    Trace,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_correctness() {
        let settings = Config::from_path(".config.yaml").unwrap();
        let template = Config {
            server: Server {
                bind_ip: Ipv4Addr::new(127, 0, 0, 1),
                port: 3000,
                events_endpoint: "github-events".to_string(),
            },
            logging: Logging {
                level: log::LevelFilter::Debug,
                file: STDERR_LOG_FILE.to_string(),
            },
            github: GitHub {
                app_id: "123456".to_string(),
                app_key_path: "./private-key.pem".to_string(),
                webhook_secret: "iseedeadpeople".to_string(),
            },
        };
        assert_eq!(settings, template);
    }
}
