//! Command-line interface definition for shiki.
//!
//! This module defines the CLI structure using clap derive macros,
//! including all subcommands and their arguments.

use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

/// shiki - Lightweight service coordination agent
///
/// A tool for coordinating service startup order across multiple machines
/// and containers via HTTP.
#[derive(Debug, Parser)]
#[command(name = "shiki")]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    /// Path to configuration file
    #[arg(short, long, global = true, env = "SHIKI_CONFIG")]
    pub config: Option<PathBuf>,

    /// Increase verbosity (can be repeated: -v, -vv, -vvv)
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    /// Suppress all output except errors
    #[arg(short, long, global = true, conflicts_with = "verbose")]
    pub quiet: bool,

    /// Subcommand to execute
    #[command(subcommand)]
    pub command: Commands,
}

impl Cli {
    /// Returns the effective log level based on verbose/quiet flags.
    /// Returns: (level_name, is_quiet)
    pub fn log_level(&self) -> (&'static str, bool) {
        if self.quiet {
            return ("error", true);
        }

        let level = match self.verbose {
            0 => "info",
            1 => "debug",
            _ => "trace",
        };

        (level, false)
    }
}

/// Available subcommands.
#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Start the HTTP server and run as an agent
    Serve(ServeArgs),

    /// Send a notification to a remote agent
    Notify(NotifyArgs),

    /// Wait for a remote service to become available
    Wait(WaitArgs),

    /// Check the status of an agent or service
    Status(StatusArgs),

    /// Configuration file operations
    #[command(subcommand)]
    Config(ConfigCommands),
}

/// Arguments for the `serve` subcommand.
#[derive(Debug, Args)]
pub struct ServeArgs {
    /// Bind address
    #[arg(long, default_value = "0.0.0.0")]
    pub bind: String,

    /// Listen port
    #[arg(long, default_value = "8080")]
    pub port: u16,
}

/// Arguments for the `notify` subcommand.
#[derive(Debug, Args)]
pub struct NotifyArgs {
    /// Target agent address (host:port)
    #[arg(short, long)]
    pub target: String,

    /// Action to perform
    #[arg(short, long, value_parser = parse_action)]
    pub action: ServiceAction,

    /// Target service name
    #[arg(short, long)]
    pub service: String,

    /// Wait for operation to complete
    #[arg(short, long, default_value = "true")]
    pub wait: bool,

    /// Timeout in seconds
    #[arg(long, default_value = "60")]
    pub timeout: u64,

    /// Do not wait for completion
    #[arg(long, conflicts_with = "wait")]
    pub no_wait: bool,
}

impl NotifyArgs {
    /// Returns whether to wait for completion.
    pub fn should_wait(&self) -> bool {
        !self.no_wait && self.wait
    }
}

/// Arguments for the `wait` subcommand.
#[derive(Debug, Args)]
pub struct WaitArgs {
    /// Target agent address (host:port)
    #[arg(short, long)]
    pub target: String,

    /// Service name to wait for
    #[arg(short, long)]
    pub service: String,

    /// Timeout in seconds
    #[arg(long, default_value = "60")]
    pub timeout: u64,

    /// Polling interval in seconds
    #[arg(long, default_value = "5")]
    pub interval: u64,
}

/// Arguments for the `status` subcommand.
#[derive(Debug, Args)]
pub struct StatusArgs {
    /// Remote agent address (if checking remote status)
    #[arg(long)]
    pub target: Option<String>,

    /// Service name (if checking service status)
    #[arg(long)]
    pub service: Option<String>,
}

/// Configuration subcommands.
#[derive(Debug, Subcommand)]
pub enum ConfigCommands {
    /// Validate the configuration file
    Validate,

    /// Show the current configuration
    Show,
}

/// Service action types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceAction {
    /// Start the service
    Start,
    /// Stop the service
    Stop,
    /// Restart the service
    Restart,
}

impl std::fmt::Display for ServiceAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServiceAction::Start => write!(f, "start"),
            ServiceAction::Stop => write!(f, "stop"),
            ServiceAction::Restart => write!(f, "restart"),
        }
    }
}

impl std::str::FromStr for ServiceAction {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "start" => Ok(ServiceAction::Start),
            "stop" => Ok(ServiceAction::Stop),
            "restart" => Ok(ServiceAction::Restart),
            _ => Err(format!(
                "Invalid action '{}'. Valid actions: start, stop, restart",
                s
            )),
        }
    }
}

/// Parse service action from string.
fn parse_action(s: &str) -> Result<ServiceAction, String> {
    s.parse()
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn test_cli_debug() {
        // Verify CLI can be constructed
        Cli::command().debug_assert();
    }

    #[test]
    fn test_serve_command() {
        let cli = Cli::parse_from(["shiki", "serve"]);

        match cli.command {
            Commands::Serve(args) => {
                assert_eq!(args.bind, "0.0.0.0");
                assert_eq!(args.port, 8080);
            }
            _ => panic!("Expected Serve command"),
        }
    }

    #[test]
    fn test_serve_with_args() {
        let cli = Cli::parse_from(["shiki", "serve", "--bind", "127.0.0.1", "--port", "9090"]);

        match cli.command {
            Commands::Serve(args) => {
                assert_eq!(args.bind, "127.0.0.1");
                assert_eq!(args.port, 9090);
            }
            _ => panic!("Expected Serve command"),
        }
    }

    #[test]
    fn test_notify_command() {
        let cli = Cli::parse_from([
            "shiki",
            "notify",
            "-t",
            "localhost:8080",
            "-a",
            "start",
            "-s",
            "nginx",
        ]);

        match cli.command {
            Commands::Notify(args) => {
                assert_eq!(args.target, "localhost:8080");
                assert_eq!(args.action, ServiceAction::Start);
                assert_eq!(args.service, "nginx");
                assert!(args.should_wait());
                assert_eq!(args.timeout, 60);
            }
            _ => panic!("Expected Notify command"),
        }
    }

    #[test]
    fn test_notify_no_wait() {
        let cli = Cli::parse_from([
            "shiki",
            "notify",
            "-t",
            "localhost:8080",
            "-a",
            "start",
            "-s",
            "nginx",
            "--no-wait",
        ]);

        match cli.command {
            Commands::Notify(args) => {
                assert!(!args.should_wait());
            }
            _ => panic!("Expected Notify command"),
        }
    }

    #[test]
    fn test_wait_command() {
        let cli = Cli::parse_from([
            "shiki",
            "wait",
            "-t",
            "db.local:8080",
            "-s",
            "postgres",
            "--timeout",
            "120",
            "--interval",
            "10",
        ]);

        match cli.command {
            Commands::Wait(args) => {
                assert_eq!(args.target, "db.local:8080");
                assert_eq!(args.service, "postgres");
                assert_eq!(args.timeout, 120);
                assert_eq!(args.interval, 10);
            }
            _ => panic!("Expected Wait command"),
        }
    }

    #[test]
    fn test_status_command_local() {
        let cli = Cli::parse_from(["shiki", "status", "--service", "nginx"]);

        match cli.command {
            Commands::Status(args) => {
                assert!(args.target.is_none());
                assert_eq!(args.service, Some("nginx".to_string()));
            }
            _ => panic!("Expected Status command"),
        }
    }

    #[test]
    fn test_status_command_remote() {
        let cli = Cli::parse_from(["shiki", "status", "--target", "remote:8080"]);

        match cli.command {
            Commands::Status(args) => {
                assert_eq!(args.target, Some("remote:8080".to_string()));
                assert!(args.service.is_none());
            }
            _ => panic!("Expected Status command"),
        }
    }

    #[test]
    fn test_config_validate() {
        let cli = Cli::parse_from(["shiki", "config", "validate"]);

        match cli.command {
            Commands::Config(ConfigCommands::Validate) => {}
            _ => panic!("Expected Config Validate command"),
        }
    }

    #[test]
    fn test_config_show() {
        let cli = Cli::parse_from(["shiki", "config", "show"]);

        match cli.command {
            Commands::Config(ConfigCommands::Show) => {}
            _ => panic!("Expected Config Show command"),
        }
    }

    #[test]
    fn test_global_config_option() {
        let cli = Cli::parse_from(["shiki", "-c", "/custom/config.yaml", "serve"]);

        assert_eq!(cli.config, Some(PathBuf::from("/custom/config.yaml")));
    }

    #[test]
    fn test_verbose_levels() {
        let cli = Cli::parse_from(["shiki", "serve"]);
        assert_eq!(cli.log_level(), ("info", false));

        let cli = Cli::parse_from(["shiki", "-v", "serve"]);
        assert_eq!(cli.log_level(), ("debug", false));

        let cli = Cli::parse_from(["shiki", "-vv", "serve"]);
        assert_eq!(cli.log_level(), ("trace", false));

        let cli = Cli::parse_from(["shiki", "-vvv", "serve"]);
        assert_eq!(cli.log_level(), ("trace", false));
    }

    #[test]
    fn test_quiet_mode() {
        let cli = Cli::parse_from(["shiki", "-q", "serve"]);
        assert_eq!(cli.log_level(), ("error", true));
    }

    #[test]
    fn test_service_action_parse() {
        assert_eq!(
            "start".parse::<ServiceAction>().unwrap(),
            ServiceAction::Start
        );
        assert_eq!(
            "STOP".parse::<ServiceAction>().unwrap(),
            ServiceAction::Stop
        );
        assert_eq!(
            "Restart".parse::<ServiceAction>().unwrap(),
            ServiceAction::Restart
        );
        assert!("invalid".parse::<ServiceAction>().is_err());
    }

    #[test]
    fn test_service_action_display() {
        assert_eq!(format!("{}", ServiceAction::Start), "start");
        assert_eq!(format!("{}", ServiceAction::Stop), "stop");
        assert_eq!(format!("{}", ServiceAction::Restart), "restart");
    }
}
