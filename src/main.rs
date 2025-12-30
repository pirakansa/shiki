//! shiki - Lightweight service coordination agent
//!
//! Entry point for the shiki application.

use clap::Parser;
use shiki::cli::{Cli, Commands, ConfigCommands};
use shiki::config::Config;
use shiki::error::exit_code;
use std::process::ExitCode;
use tracing::Level;
use tracing_subscriber::fmt::format::FmtSpan;

fn main() -> ExitCode {
    let cli = Cli::parse();

    // Initialize logging based on CLI flags
    if let Err(e) = init_logging(&cli) {
        eprintln!("Failed to initialize logging: {}", e);
        return ExitCode::from(exit_code::GENERAL_ERROR as u8);
    }

    // Execute the command
    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            tracing::error!("{}", e);
            ExitCode::from(e.exit_code() as u8)
        }
    }
}

/// Initialize the tracing subscriber based on CLI options.
fn init_logging(cli: &Cli) -> Result<(), Box<dyn std::error::Error>> {
    let (level_str, _is_quiet) = cli.log_level();

    let level = match level_str {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    };

    // For now, use text format by default in CLI
    // The JSON format will be configured from config file in serve mode
    tracing_subscriber::fmt()
        .with_max_level(level)
        .with_span_events(FmtSpan::CLOSE)
        .with_target(true)
        .init();

    Ok(())
}

/// Main application logic.
fn run(cli: Cli) -> shiki::Result<()> {
    match &cli.command {
        Commands::Serve(args) => cmd_serve(&cli, args),
        Commands::Notify(args) => cmd_notify(&cli, args),
        Commands::Wait(args) => cmd_wait(&cli, args),
        Commands::Status(args) => cmd_status(&cli, args),
        Commands::Config(subcmd) => cmd_config(&cli, subcmd),
    }
}

/// Handle the `serve` command.
fn cmd_serve(cli: &Cli, args: &shiki::cli::ServeArgs) -> shiki::Result<()> {
    let mut config = load_config(cli)?;

    // Use CLI args if provided, otherwise fall back to config
    if args.bind != "0.0.0.0" {
        config.server.bind = args.bind.clone();
    }

    if args.port != 8080 {
        config.server.port = args.port;
    }

    tracing::info!(
        agent_name = %config.agent_name(),
        backend = ?config.agent.backend,
        bind = %config.server.bind,
        port = %config.server.port,
        "Starting shiki server"
    );

    // Create tokio runtime and run the server
    let runtime = tokio::runtime::Runtime::new().map_err(|e| {
        shiki::ShikiError::backend_with_source("Failed to create async runtime".to_string(), e)
    })?;

    runtime.block_on(async { shiki::serve(&config).await })
}

/// Handle the `notify` command.
fn cmd_notify(_cli: &Cli, args: &shiki::cli::NotifyArgs) -> shiki::Result<()> {
    let service_action = match args.action {
        shiki::cli::ServiceAction::Start => shiki::service::ServiceAction::Start,
        shiki::cli::ServiceAction::Stop => shiki::service::ServiceAction::Stop,
        shiki::cli::ServiceAction::Restart => shiki::service::ServiceAction::Restart,
    };

    tracing::info!(
        target = %args.target,
        action = %args.action,
        service = %args.service,
        wait = %args.should_wait(),
        "Sending notification"
    );

    let runtime = tokio::runtime::Runtime::new().map_err(|e| {
        shiki::ShikiError::backend_with_source("Failed to create async runtime".to_string(), e)
    })?;

    runtime.block_on(async {
        let client = shiki::ShikiClient::new(&args.target)?;
        let result = client
            .notify(
                &args.service,
                service_action,
                args.should_wait(),
                args.timeout,
            )
            .await?;

        println!("Request ID: {}", result.request_id);
        println!("Service: {}", result.service);
        println!("Action: {}", result.action);
        println!("Result: {}", result.result);

        if let Some(prev) = &result.previous_status {
            println!("Previous Status: {}", prev);
        }
        if let Some(curr) = &result.current_status {
            println!("Current Status: {}", curr);
        }
        if let Some(dur) = result.duration_ms {
            println!("Duration: {}ms", dur);
        }
        if let Some(msg) = &result.message {
            println!("Message: {}", msg);
        }

        Ok(())
    })
}

/// Handle the `wait` command.
fn cmd_wait(_cli: &Cli, args: &shiki::cli::WaitArgs) -> shiki::Result<()> {
    tracing::info!(
        target = %args.target,
        service = %args.service,
        timeout = %args.timeout,
        interval = %args.interval,
        "Waiting for service"
    );

    let runtime = tokio::runtime::Runtime::new().map_err(|e| {
        shiki::ShikiError::backend_with_source("Failed to create async runtime".to_string(), e)
    })?;

    runtime.block_on(async {
        let client = shiki::ShikiClient::new(&args.target)?;
        let timeout = std::time::Duration::from_secs(args.timeout);
        let interval = std::time::Duration::from_secs(args.interval);

        // Default to waiting for "running" status
        let target_status = "running";

        client
            .wait_for_service(&args.service, target_status, timeout, interval)
            .await?;

        println!("Service '{}' is now {}", args.service, target_status);
        Ok(())
    })
}

/// Handle the `status` command.
fn cmd_status(cli: &Cli, args: &shiki::cli::StatusArgs) -> shiki::Result<()> {
    if let Some(target) = &args.target {
        // Remote status check
        tracing::info!(target = %target, "Checking remote agent status");

        let runtime = tokio::runtime::Runtime::new().map_err(|e| {
            shiki::ShikiError::backend_with_source("Failed to create async runtime".to_string(), e)
        })?;

        runtime.block_on(async {
            let client = shiki::ShikiClient::new(target)?;
            let status = client.status().await?;

            println!("Remote Agent Status");
            println!("===================");
            println!("Name: {}", status.agent.name);
            println!("State: {:?}", status.agent.state);
            println!("Mode: {}", status.agent.mode);
            println!("Server: {}:{}", status.server.bind, status.server.port);
            println!(
                "TLS: {}",
                if status.server.tls_enabled {
                    "enabled"
                } else {
                    "disabled"
                }
            );
            println!("Version: {}", status.version);
            println!("Uptime: {}s", status.uptime_seconds);
            println!("\nStatistics:");
            println!("  Total Requests: {}", status.stats.requests_total);
            println!("  Successful: {}", status.stats.requests_success);
            println!("  Failed: {}", status.stats.requests_failed);

            if !status.agent.tags.is_empty() {
                println!("\nTags: {}", status.agent.tags.join(", "));
            }

            Ok(())
        })
    } else if let Some(service) = &args.service {
        // Local service status check
        let config = load_config(cli)?;
        tracing::info!(service = %service, "Checking local service status");

        let runtime = tokio::runtime::Runtime::new().map_err(|e| {
            shiki::ShikiError::backend_with_source("Failed to create async runtime".to_string(), e)
        })?;

        runtime.block_on(async {
            let controller = shiki::ServiceController::from_config(&config)?;
            let status = controller.status(service).await?;

            println!("Service: {}", status.name);
            println!("Status: {}", status.state);
            if let Some(desc) = &status.description {
                println!("Description: {}", desc);
            }

            Ok(())
        })
    } else {
        // Local agent status
        let config = load_config(cli)?;
        println!("Agent Status");
        println!("============");
        println!("Name: {}", config.agent_name());
        println!("Mode: {:?}", config.agent.mode);
        println!("Backend: {:?}", config.agent.backend);
        println!("Server: {}:{}", config.server.bind, config.server.port);
        println!(
            "TLS: {}",
            if config.server.tls.enabled {
                "enabled"
            } else {
                "disabled"
            }
        );
        println!(
            "Auth: {}",
            if config.auth.enabled {
                "enabled"
            } else {
                "disabled"
            }
        );

        if !config.agent.tags.is_empty() {
            println!("Tags: {}", config.agent.tags.join(", "));
        }

        if !config.services.is_empty() {
            println!("\nConfigured Services:");
            for name in config.services.keys() {
                println!("  - {}", name);
            }
        }

        Ok(())
    }
}

/// Handle the `config` subcommand.
fn cmd_config(cli: &Cli, subcmd: &ConfigCommands) -> shiki::Result<()> {
    match subcmd {
        ConfigCommands::Validate => {
            let config_path = cli.config.as_deref();
            match Config::load(config_path) {
                Ok(config) => {
                    println!("✓ Configuration is valid");
                    tracing::debug!(?config, "Validated configuration");
                    Ok(())
                }
                Err(e) => {
                    println!("✗ Configuration is invalid: {}", e);
                    Err(e)
                }
            }
        }
        ConfigCommands::Show => {
            let config = load_config(cli)?;
            let yaml = serde_yaml::to_string(&config).map_err(|e| {
                shiki::ShikiError::config_with_source("Failed to serialize configuration", e)
            })?;
            println!("{}", yaml);
            Ok(())
        }
    }
}

/// Load configuration with error handling.
fn load_config(cli: &Cli) -> shiki::Result<Config> {
    let config_path = cli.config.as_deref();
    Config::load(config_path)
}
