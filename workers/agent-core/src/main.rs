mod capability;
mod protocol;

use std::env;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, PartialEq, Eq)]
struct WorkerConfig {
    worker_id: String,
    role: String,
    capabilities: Vec<String>,
}

fn main() {
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

    println!("Hermes worker started");
    println!("worker_id: {}", config.worker_id);
    println!("role: {}", config.role);
    println!("capabilities:");
    for capability in config.capabilities {
        println!("- {capability}");
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
            unknown => return Err(format!("unknown configuration key: {unknown}")),
        }
    }

    Ok(WorkerConfig {
        worker_id: required_value("worker_id", worker_id)?,
        role: required_value("role", role)?,
        capabilities: capabilities
            .ok_or_else(|| "missing required key: capabilities".to_string())?,
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
            "worker_id=windows-dev\nrole=development\ncapabilities=system.info, files.read\n",
        )
        .unwrap();
        assert_eq!(
            config,
            WorkerConfig {
                worker_id: "windows-dev".to_string(),
                role: "development".to_string(),
                capabilities: vec!["system.info".to_string(), "files.read".to_string()],
            }
        );
    }

    #[test]
    fn rejects_unknown_keys() {
        let error = parse_config("worker_id=a\nrole=b\ncapabilities=system.info\nbrain_url=x\n")
            .unwrap_err();
        assert_eq!(error, "unknown configuration key: brain_url");
    }
}
