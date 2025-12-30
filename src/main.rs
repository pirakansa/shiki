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
    let config = load_config(cli)?;

    // Use CLI args if provided, otherwise fall back to config
    let bind = if args.bind != "0.0.0.0" {
        args.bind.clone()
    } else {
        config.server.bind.clone()
    };

    let port = if args.port != 8080 {
        args.port
    } else {
        config.server.port
    };

    tracing::info!(
        agent_name = %config.agent_name(),
        backend = ?config.agent.backend,
        bind = %bind,
        port = %port,
        "Starting shiki server"
    );

    // TODO: Implement HTTP server (Phase 3)
    tracing::warn!("HTTP server not yet implemented");
    println!("Server would start on {}:{}", bind, port);
    println!("Agent name: {}", config.agent_name());
    println!("Backend: {:?}", config.agent.backend);

    Ok(())
}

/// Handle the `notify` command.
fn cmd_notify(_cli: &Cli, args: &shiki::cli::NotifyArgs) -> shiki::Result<()> {
    tracing::info!(
        target = %args.target,
        action = %args.action,
        service = %args.service,
        wait = %args.should_wait(),
        "Sending notification"
    );

    // TODO: Implement HTTP client notify (Phase 4)
    tracing::warn!("Notify command not yet implemented");
    println!(
        "Would notify {} to {} service {} (wait={})",
        args.target,
        args.action,
        args.service,
        args.should_wait()
    );

    Ok(())
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

    // TODO: Implement wait polling (Phase 4)
    tracing::warn!("Wait command not yet implemented");
    println!(
        "Would wait for {} on {} (timeout={}s, interval={}s)",
        args.service, args.target, args.timeout, args.interval
    );

    Ok(())
}

/// Handle the `status` command.
fn cmd_status(cli: &Cli, args: &shiki::cli::StatusArgs) -> shiki::Result<()> {
    if let Some(target) = &args.target {
        // Remote status check
        tracing::info!(target = %target, "Checking remote agent status");
        // TODO: Implement remote status check (Phase 4)
        tracing::warn!("Remote status check not yet implemented");
        println!("Would check status of remote agent: {}", target);
    } else if let Some(service) = &args.service {
        // Local service status check
        let config = load_config(cli)?;
        tracing::info!(service = %service, "Checking local service status");
        // TODO: Implement local service status check (Phase 2)
        tracing::warn!("Local service status check not yet implemented");
        println!(
            "Would check status of local service '{}' using {:?} backend",
            service, config.agent.backend
        );
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
    }

    Ok(())
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
