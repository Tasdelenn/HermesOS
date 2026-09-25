mod capability;
mod client;
mod state;

use std::env;
use std::fs;
use std::path::PathBuf;

use client::WorkerClient;

#[derive(Debug, PartialEq, Eq)]
pub struct WorkerConfig {
    pub worker_id: String,
    pub role: String,
    pub capabilities: Vec<String>,
    pub brain_url: String,
    pub registration_token: Option<String>,
    pub heartbeat_interval_secs: u64,
    /// Where the enrollment credential is stored. Defaults to
    /// `<config-dir>/<config-stem>.state.json` (see `state::default_state_path`).
    pub state_file: Option<PathBuf>,
}

#[tokio::main]
async fn main() {
    let config_path = match config_path_from_args() {
        Ok(path) => path,
        Err(error) => exit_with_error(&error),
    };

    let contents = fs::read_to_string(&config_path).unwrap_or_else(|error| {
        exit_with_error(&format!(
            "could not read configuration at {}: {error}",
            config_path.display()
        ))
    });
    let config = parse_config(&contents).unwrap_or_else(|error| exit_with_error(&error));

    println!("Hermes worker starting");
    println!("worker_id: {}", config.worker_id);
    println!("role: {}", config.role);
    println!("brain_url: {}", config.brain_url);

    let state_path = config
        .state_file
        .clone()
        .unwrap_or_else(|| state::default_state_path(&config_path));
    println!("state_file: {}", state_path.display());

    let mut client = WorkerClient::connect(config, state_path)
        .await
        .unwrap_or_else(|error| exit_with_error(&error));

    if let Err(error) = client.run().await {
        exit_with_error(&error);
    }
}

fn config_path_from_args() -> Result<PathBuf, String> {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        None => Ok(PathBuf::from("config/worker.conf")),
        Some("--config") => match (args.next(), args.next()) {
            (Some(path), None) => Ok(PathBuf::from(path)),
            _ => Err("usage: hermes-worker [--config <path>]".to_string()),
        },
        Some("--help") | Some("-h") => {
            println!("usage: hermes-worker [--config <path>]");
            std::process::exit(0);
        }
        Some(_) => Err("usage: hermes-worker [--config <path>]".to_string()),
    }
}

fn parse_config(contents: &str) -> Result<WorkerConfig, String> {
    let mut worker_id = None;
    let mut role = None;
    let mut capabilities = None;
    let mut brain_url = None;
    let mut registration_token = None;
    let mut heartbeat_interval_secs = None;
    let mut state_file = None;

    for (index, line) in contents.lines().enumerate() {
        let line_number = index + 1;
        let line = line.split_once('#').map_or(line, |(value, _)| value).trim();
        if line.is_empty() {
            continue;
        }

        let (key, value) = line.split_once('=').ok_or_else(|| {
            format!("invalid configuration on line {line_number}: expected key=value")
        })?;
        let value = value.trim();
        if value.is_empty() {
            return Err(format!(
                "invalid configuration on line {line_number}: value is empty"
            ));
        }

        match key.trim() {
            "worker_id" => worker_id = Some(value.to_owned()),
            "role" => role = Some(value.to_owned()),
            "capabilities" => {
                let parsed = value
                    .split(',')
                    .map(str::trim)
                    .filter(|item| !item.is_empty())
                    .map(str::to_owned)
                    .collect::<Vec<_>>();
                if parsed.is_empty() {
                    return Err("capabilities must contain at least one capability".to_string());
                }
                capabilities = Some(parsed);
            }
            "brain_url" => brain_url = Some(value.to_owned()),
            "registration_token" => registration_token = Some(value.to_owned()),
            "heartbeat_interval_secs" => {
                heartbeat_interval_secs = Some(value.parse::<u64>().map_err(|_| {
                    format!("heartbeat_interval_secs must be a number, got: {value}")
                })?);
            }
            "state_file" => state_file = Some(PathBuf::from(value)),
            unknown => return Err(format!("unknown configuration key: {unknown}")),
        }
    }

    Ok(WorkerConfig {
        worker_id: required_value("worker_id", worker_id)?,
        role: required_value("role", role)?,
        capabilities: capabilities
            .ok_or_else(|| "missing required key: capabilities".to_string())?,
        brain_url: brain_url.unwrap_or_else(|| "ws://127.0.0.1:9000".to_string()),
        registration_token,
        heartbeat_interval_secs: heartbeat_interval_secs.unwrap_or(30),
        state_file,
    })
}

fn required_value(name: &str, value: Option<String>) -> Result<String, String> {
    value.ok_or_else(|| format!("missing required key: {name}"))
}

fn exit_with_error(message: &str) -> ! {
    eprintln!("hermes-worker: {message}");
    std::process::exit(1);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_config() {
        let config = parse_config(
            "worker_id=windows-dev\nrole=development\ncapabilities=system.info, files.read\nbrain_url=ws://brain:9000\nregistration_token=tok-abc\nheartbeat_interval_secs=15\n",
        )
        .unwrap();
        assert_eq!(
            config,
            WorkerConfig {
                worker_id: "windows-dev".to_string(),
                role: "development".to_string(),
                capabilities: vec!["system.info".to_string(), "files.read".to_string()],
                brain_url: "ws://brain:9000".to_string(),
                registration_token: Some("tok-abc".to_string()),
                heartbeat_interval_secs: 15,
                state_file: None,
            }
        );
    }

    #[test]
    fn uses_defaults_for_optional_fields() {
        let config =
            parse_config("worker_id=minimal\nrole=iot\ncapabilities=system.info\n").unwrap();
        assert_eq!(config.brain_url, "ws://127.0.0.1:9000");
        assert_eq!(config.registration_token, None);
        assert_eq!(config.heartbeat_interval_secs, 30);
        assert_eq!(config.state_file, None);
    }

    #[test]
    fn parses_state_file() {
        let config = parse_config(
            "worker_id=a\nrole=b\ncapabilities=system.info\nstate_file=/var/lib/hermes/a.state.json\n",
        )
        .unwrap();
        assert_eq!(
            config.state_file,
            Some(PathBuf::from("/var/lib/hermes/a.state.json"))
        );
    }

    #[test]
    fn rejects_unknown_keys() {
        let error = parse_config(
            "worker_id=a\nrole=b\ncapabilities=system.info\nbrain_url=x\nunknown_key=y\n",
        )
        .unwrap_err();
        assert_eq!(error, "unknown configuration key: unknown_key");
    }

    #[test]
    fn rejects_invalid_heartbeat() {
        let error = parse_config(
            "worker_id=a\nrole=b\ncapabilities=system.info\nheartbeat_interval_secs=not-a-number\n",
        )
        .unwrap_err();
        assert!(error.contains("must be a number"));
    }
}
